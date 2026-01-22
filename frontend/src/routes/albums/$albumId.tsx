import { createFileRoute, Link, redirect, useNavigate } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useMemo } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Chip } from '@heroui/chip'
import { Image } from '@heroui/image'
import { ShimmerLoader } from '../../components/shared/ShimmerLoader'
import { albumTemplate, libraryTemplate } from '../../lib/template-data'
import { Breadcrumbs, BreadcrumbItem } from '@heroui/breadcrumbs'
import { Progress } from '@heroui/progress'
import { useDisclosure } from '@heroui/modal'
import { RouteError } from '../../components/RouteError'
import { sanitizeError, formatBytes, formatDuration } from '../../lib/format'
import {
  graphqlClient,
  ALBUM_WITH_TRACKS_QUERY,
  LIBRARY_QUERY,
} from '../../lib/graphql'
import type {
  AlbumWithTracks,
  Library,
  TrackWithStatus,
} from '../../lib/graphql'
import { DataTable, type DataTableColumn, type RowAction } from '../../components/data-table'
import {
  IconDisc,
  IconMusic,
  IconSearch,
  IconRefresh,
  IconPlayerPlay,
  IconPlayerPause,
  IconInfoCircle,
} from '@tabler/icons-react'
// Note: IconPlayerPause is used for artwork overlay
import { FilePropertiesModal } from '../../components/FilePropertiesModal'
import { TrackStatusChip, PlayPauseIndicator } from '../../components/shared'
import { usePlaybackContext } from '../../contexts/PlaybackContext'
import { useDataReactivity } from '../../hooks/useSubscription'

export const Route = createFileRoute('/albums/$albumId')({
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
  component: AlbumDetailPage,
  errorComponent: RouteError,
})

// Helper to format audio codec display name
function formatAudioCodec(codec: string | null): string {
  if (!codec) return ''
  const normalized = codec.toLowerCase()
  if (normalized.includes('flac')) return 'FLAC'
  if (normalized.includes('alac')) return 'ALAC'
  if (normalized.includes('aac')) return 'AAC'
  if (normalized.includes('mp3') || normalized.includes('mpeg')) return 'MP3'
  if (normalized.includes('opus')) return 'Opus'
  if (normalized.includes('vorbis')) return 'Vorbis'
  if (normalized.includes('wav') || normalized.includes('pcm')) return 'WAV'
  return codec.toUpperCase()
}

// Helper to format bitrate
function formatBitrate(bitrate: number | null): string {
  if (!bitrate) return ''
  if (bitrate >= 1000) {
    return `${(bitrate / 1000).toFixed(0)} Mbps`
  }
  return `${bitrate} kbps`
}

// Track table columns
const trackColumns: DataTableColumn<TrackWithStatus>[] = [
  {
    key: 'trackNumber',
    label: '#',
    width: 60,
    sortable: true,
    render: (t) => (
      <span className="font-mono text-default-500">
        {t.track.discNumber > 1 && `${t.track.discNumber}-`}
        {String(t.track.trackNumber).padStart(2, '0')}
      </span>
    ),
  },
  {
    key: 'title',
    label: 'Title',
    sortable: true,
    render: (t) => (
      <div className="flex flex-col">
        <span className="font-medium">{t.track.title}</span>
        {t.track.artistName && (
          <span className="text-xs text-default-400">{t.track.artistName}</span>
        )}
      </div>
    ),
  },
  {
    key: 'duration',
    label: 'Duration',
    width: 80,
    render: (t) => (
      <span className="text-default-500 text-sm">
        {t.track.durationSecs ? formatDuration(t.track.durationSecs) : '-'}
      </span>
    ),
  },
  {
    key: 'quality',
    label: 'Quality',
    width: 150,
    render: (t) => {
      if (!t.hasFile || !t.track.mediaFileId) {
        return <span className="text-default-400">-</span>
      }
      return (
        <div className="flex items-center gap-1.5 flex-wrap">
          {t.audioCodec && (
            <Chip size="sm" variant="flat" color="primary" className="h-5 text-xs">
              {formatAudioCodec(t.audioCodec)}
            </Chip>
          )}
          {t.bitrate && (
            <Chip size="sm" variant="flat" color="secondary" className="h-5 text-xs">
              {formatBitrate(t.bitrate)}
            </Chip>
          )}
        </div>
      )
    },
  },
  {
    key: 'size',
    label: 'Size',
    width: 100,
    render: (t) => {
      if (!t.hasFile || !t.fileSize) {
        return <span className="text-default-400">-</span>
      }
      return (
        <span className="text-default-500 text-sm text-nowrap">
          {formatBytes(t.fileSize)}
        </span>
      )
    },
  },
  {
    key: 'status',
    label: 'Status',
    width: 140,
    sortable: true,
    render: (t) => (
      <TrackStatusChip
        mediaFileId={t.track.mediaFileId}
        downloadProgress={t.track.downloadProgress}
      />
    ),
  },
]

interface TrackTableProps {
  tracks: TrackWithStatus[]
  albumId: string
  onPlay: (track: TrackWithStatus) => void
  onSearch: (track: TrackWithStatus) => void
  onShowProperties: (track: TrackWithStatus) => void
  fetchAlbum: () => void
}

function TrackTable({ tracks, albumId, onPlay, onSearch, onShowProperties, fetchAlbum }: TrackTableProps) {
  // Get session and updatePlayback directly from context for reliable updates
  const { session, updatePlayback } = usePlaybackContext()

  // Handle pause directly using context
  const handlePause = useCallback(() => {
    updatePlayback({ isPlaying: false })
  }, [updatePlayback])

  // Compute playing state from session
  const currentlyPlayingTrackId = session?.albumId === albumId ? session?.trackId : null
  const isPlaying = session?.isPlaying ?? false
  // Row actions - computed fresh on each render to ensure playing state is always current
  const rowActions: RowAction<TrackWithStatus>[] = [
    // Playing indicator with pause on hover - shown for currently playing track
    {
      key: `playing-${currentlyPlayingTrackId || 'none'}`,
      label: 'Pause',
      icon: <PlayPauseIndicator size={16} isPlaying={isPlaying} colorClass="bg-success" />,
      color: 'default',
      inDropdown: false,
      isVisible: (t) => t.track.status === 'downloaded' && !!t.track.mediaFileId && currentlyPlayingTrackId === t.track.id && isPlaying,
      onAction: () => handlePause(),
    },
    // Play action - shown for all other tracks or when paused
    {
      key: `play-${currentlyPlayingTrackId || 'none'}-${isPlaying}`,
      label: 'Play',
      icon: <IconPlayerPlay size={16} />,
      color: 'success',
      inDropdown: false,
      isVisible: (t) => t.track.status === 'downloaded' && !!t.track.mediaFileId && !(currentlyPlayingTrackId === t.track.id && isPlaying),
      onAction: (t) => onPlay(t),
    },
    {
      key: 'search',
      label: 'Search for Track',
      icon: <IconSearch size={16} />,
      color: 'default',
      inDropdown: false,
      // Show search for missing or wanted tracks (not downloading or downloaded)
      isVisible: (t) => t.track.status === 'missing' || t.track.status === 'wanted',
      onAction: (t) => onSearch(t),
    },
    {
      key: 'properties',
      label: 'File Properties',
      icon: <IconInfoCircle size={16} />,
      color: 'default',
      inDropdown: true,
      // Only show properties for downloaded tracks with a media file
      isVisible: (t) => t.track.status === 'downloaded' && !!t.track.mediaFileId,
      onAction: (t) => onShowProperties(t),
    },
  ]

  // Create selection set for highlighting currently playing track
  const selectedKeys = useMemo(() => {
    if (currentlyPlayingTrackId) {
      return new Set([currentlyPlayingTrackId])
    }
    return new Set<string>()
  }, [currentlyPlayingTrackId])

  // Key that changes when playback state changes to force re-render
  const tableKey = `tracks-${currentlyPlayingTrackId || 'none'}-${isPlaying}`

  return (
    <DataTable
      headerContent={
        <div className="p-4">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <IconMusic size={20} className="text-green-400" />
            Tracks
          </h2>
        </div>
      }
      key={tableKey}
      data={tracks}
      columns={trackColumns}
      emptyContent={<div className="p-8 text-center">
        <IconMusic size={48} className="mx-auto mb-4 text-default-400" />
        <h3 className="text-lg font-semibold mb-2">No Tracks</h3>
        <p className="text-default-500 mb-4">
          Track information hasn't been fetched yet.
        </p>
        <Button variant="flat" onPress={fetchAlbum}>
          Refresh Album
        </Button>
      </div>
      }
      getRowKey={(t) => t.track.id}
      ariaLabel="Album tracks"
      isCompact
      showItemCount={false}
      hideToolbar
      defaultSortColumn="trackNumber"
      defaultSortDirection="asc"
      rowActions={rowActions}
      selectionMode={currentlyPlayingTrackId ? 'single' : 'none'}
      selectedKeys={selectedKeys}
    />
  )
}

function AlbumDetailPage() {
  const { albumId } = Route.useParams()
  const navigate = useNavigate()
  const { startTrackPlayback, session, updatePlayback } = usePlaybackContext()

  // Check if this album is currently playing
  const isThisAlbumPlaying = session?.albumId === albumId && session?.isPlaying

  const [albumData, setAlbumData] = useState<AlbumWithTracks | null>(null)
  const [library, setLibrary] = useState<Library | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const { isOpen: isPropertiesOpen, onOpen: onPropertiesOpen, onClose: onPropertiesClose } = useDisclosure()
  const [propertiesTrack, setPropertiesTrack] = useState<TrackWithStatus | null>(null)

  // Fetch album data
  const fetchAlbum = useCallback(async () => {
    try {
      const result = await graphqlClient
        .query<{ albumWithTracks: AlbumWithTracks | null }>(ALBUM_WITH_TRACKS_QUERY, { id: albumId })
        .toPromise()

      if (result.error) {
        throw new Error(sanitizeError(result.error.message))
      }

      if (result.data?.albumWithTracks) {
        setAlbumData(result.data.albumWithTracks)

        // Fetch library info
        const libResult = await graphqlClient
          .query<{ library: Library | null }>(LIBRARY_QUERY, { id: result.data.albumWithTracks.album.libraryId })
          .toPromise()
        if (libResult.data?.library) {
          setLibrary(libResult.data.library)
        }
      } else {
        setError('Album not found')
      }
    } catch (e) {
      setError(sanitizeError(e))
    } finally {
      setIsLoading(false)
    }
  }, [albumId])

  useEffect(() => {
    fetchAlbum()
  }, [fetchAlbum])

  // Keep data fresh with periodic updates and torrent completion events
  // This ensures download progress is updated in real-time
  useDataReactivity(fetchAlbum, {
    onTorrentComplete: true,
    periodicInterval: 10000, // Refresh every 10 seconds to match backend sync interval
    onFocus: true,
  })

  // Navigate to hunt page for this album
  const handleManualHunt = useCallback(() => {
    if (!albumData) return
    const searchQuery = `${albumData.album.name}`
    navigate({
      to: '/hunt',
      search: { q: searchQuery, type: 'music', albumId: albumData.album.id },
    })
  }, [albumData, navigate])

  // Handle play track - start playback with the audio player
  const handlePlayTrack = useCallback((track: TrackWithStatus) => {
    if (track.track.mediaFileId && albumData) {
      // Get all tracks that have media files for the queue
      const allTracks = albumData.tracks
        .filter(t => t.track.mediaFileId)
        .map(t => t.track)

      startTrackPlayback(track.track, albumData.album, allTracks)
    }
  }, [albumData, startTrackPlayback])

  // Note: Playing state and pause handling are now inside TrackTable component directly from context

  // Navigate to hunt page for a specific track
  const handleSearchTrack = useCallback((track: TrackWithStatus) => {
    if (!albumData) return
    // Build search query: "Artist - Track Title"
    const artist = track.track.artistName || albumData.album.name
    const searchQuery = `${artist} ${track.track.title}`
    navigate({
      to: '/hunt',
      search: {
        q: searchQuery,
        type: 'music',
        albumId: albumData.album.id,
      },
    })
  }, [albumData, navigate])

  // Show file properties modal for a track
  const handleShowProperties = useCallback((track: TrackWithStatus) => {
    setPropertiesTrack(track)
    onPropertiesOpen()
  }, [onPropertiesOpen])

  // Calculate totals from tracks if not available on album
  const totalDurationSecs = useMemo(() => {
    if (!albumData) return 0
    if (albumData.album.totalDurationSecs) return albumData.album.totalDurationSecs
    return albumData.tracks.reduce((sum, t) => sum + (t.track.durationSecs || 0), 0)
  }, [albumData])

  const totalSizeBytes = useMemo(() => {
    if (!albumData) return 0
    if (albumData.album.sizeBytes) return albumData.album.sizeBytes
    return albumData.tracks.reduce((sum, t) => sum + (t.fileSize || 0), 0)
  }, [albumData])

  // Get playable tracks for "Play All" functionality
  const playableTracks = useMemo(() => {
    if (!albumData) return []
    return albumData.tracks.filter(t => t.track.status === 'downloaded' && t.track.mediaFileId)
  }, [albumData])

  // Handle "Play All" - start playback from the first track
  const handlePlayAll = useCallback(() => {
    if (playableTracks.length > 0 && albumData) {
      const allTracks = playableTracks.map(t => t.track)
      startTrackPlayback(allTracks[0], albumData.album, allTracks)
    }
  }, [playableTracks, albumData, startTrackPlayback])

  // Show error state only after loading is complete
  if (!isLoading && (error || !albumData)) {
    return (
      <div className="container mx-auto p-4">
        <Card>
          <CardBody className="text-center py-12">
            <IconDisc size={48} className="mx-auto mb-4 text-default-400" />
            <h2 className="text-xl font-semibold mb-2">Album Not Found</h2>
            <p className="text-default-500 mb-4">{error || 'The album could not be loaded.'}</p>
            <Button as={Link} to="/libraries" variant="flat">
              Back to Libraries
            </Button>
          </CardBody>
        </Card>
      </div>
    )
  }

  // Use template data during loading, real data when available
  const displayAlbumData = albumData ?? albumTemplate
  const displayLibrary = library ?? libraryTemplate
  const { album, tracks } = displayAlbumData

  // Check if album is fully complete (100%)
  const isComplete = displayAlbumData.completionPercent === 100

  return (
    <ShimmerLoader loading={isLoading} templateProps={{ albumData: albumTemplate }}>
      <div className="container mx-auto p-4">
        {/* Breadcrumbs */}
        <Breadcrumbs className="mb-4">
          <BreadcrumbItem>
            <Link to="/libraries">Libraries</Link>
          </BreadcrumbItem>
          <BreadcrumbItem>
            <Link to="/libraries/$libraryId" params={{ libraryId: displayLibrary.id }}>{displayLibrary.name}</Link>
          </BreadcrumbItem>
          <BreadcrumbItem>{album.name}</BreadcrumbItem>
        </Breadcrumbs>

        {/* Album Header */}
        <div className="flex flex-col md:flex-row gap-6 mb-6">
          {/* Cover Art with Play Button */}
          <div className="w-64 shrink-0 relative group">
            {album.coverUrl ? (
              <Image
                src={album.coverUrl}
                alt={album.name}
                classNames={{
                  wrapper: 'w-64 h-64',
                  img: 'w-full h-full object-cover rounded-lg',
                }}
              />
            ) : (
              <div className="w-64 h-64 bg-content2 rounded-lg flex items-center justify-center">
                <IconDisc size={64} className="text-default-400" />
              </div>
            )}
            {/* Play/Pause Overlay Button - z-10 ensures it appears above HeroUI Image */}
            {playableTracks.length > 0 && (
              <button
                onClick={() => {
                  if (isThisAlbumPlaying) {
                    updatePlayback({ isPlaying: false })
                  } else {
                    handlePlayAll()
                  }
                }}
                className="absolute inset-0 z-10 flex items-center justify-center bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity duration-200 rounded-lg cursor-pointer"
                aria-label={isThisAlbumPlaying ? 'Pause' : 'Play All'}
              >
                <div className={`w-16 h-16 rounded-full ${isThisAlbumPlaying ? 'bg-warning' : 'bg-primary'} flex items-center justify-center shadow-lg hover:scale-110 transition-transform`}>
                  {isThisAlbumPlaying ? (
                    <IconPlayerPause size={32} className="text-white" />
                  ) : (
                    <IconPlayerPlay size={32} className="text-white ml-1" />
                  )}
                </div>
              </button>
            )}
          </div>

          {/* Album Info */}
          <div className="flex-1">
            <h1 className="text-3xl font-bold mb-1">{album.name}</h1>

            {/* Artist Name */}
            {displayAlbumData.artistName && (
              <p className="text-xl text-default-500 mb-3">{displayAlbumData.artistName}</p>
            )}

            <div className="flex flex-wrap items-center gap-2 text-default-500 mb-4">
              {album.albumType && (
                <Chip size="sm" variant="flat">
                  {album.albumType.charAt(0).toUpperCase() + album.albumType.slice(1)}
                </Chip>
              )}
              {album.year && <span>{album.year}</span>}
              {album.label && <span>• {album.label}</span>}
            </div>

            {/* Completion Progress */}
            <div className="mb-4">
              <div className="flex items-center justify-between text-sm mb-1">
                <span className="text-default-500">
                  {displayAlbumData.tracksWithFiles} of {displayAlbumData.trackCount} tracks
                </span>
                <span className="font-medium">
                  {displayAlbumData.completionPercent.toFixed(0)}%
                </span>
              </div>
              <Progress
                aria-label="Album completion"
                value={displayAlbumData.completionPercent}
                color={isComplete ? 'success' : 'primary'}
                size="sm"
              />
              {displayAlbumData.missingTracks > 0 && (
                <p className="text-sm text-warning mt-1">
                  {displayAlbumData.missingTracks} track{displayAlbumData.missingTracks !== 1 ? 's' : ''} wanted
                </p>
              )}
            </div>

            {/* Stats */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
              <div>
                <p className="text-xs text-default-400">Tracks</p>
                <p className="text-lg font-semibold">{displayAlbumData.trackCount}</p>
              </div>
              <div>
                <p className="text-xs text-default-400">Discs</p>
                <p className="text-lg font-semibold">{album.discCount || 1}</p>
              </div>
              <div>
                <p className="text-xs text-default-400">Duration</p>
                <p className="text-lg font-semibold">
                  {totalDurationSecs > 0
                    ? formatDuration(totalDurationSecs)
                    : '-'}
                </p>
              </div>
              <div>
                <p className="text-xs text-default-400">Size</p>
                <p className="text-lg font-semibold">
                  {totalSizeBytes > 0 ? formatBytes(totalSizeBytes) : '-'}
                </p>
              </div>
            </div>

            {/* Actions */}
            <div className="flex flex-wrap gap-2">
              {!isComplete && (
                <Button
                  color="primary"
                  startContent={<IconSearch size={16} />}
                  onPress={handleManualHunt}
                >
                  Hunt for Album
                </Button>
              )}
              <Button
                variant="flat"
                startContent={<IconRefresh size={16} />}
                onPress={fetchAlbum}
              >
                Refresh
              </Button>
            </div>
          </div>
        </div>

        <TrackTable
          fetchAlbum={fetchAlbum}
          tracks={tracks}
          albumId={displayAlbumData.album.id}
          onPlay={handlePlayTrack}
          onSearch={handleSearchTrack}
          onShowProperties={handleShowProperties}
        />

        {/* File Properties Modal */}
        <FilePropertiesModal
          isOpen={isPropertiesOpen}
          onClose={() => {
            onPropertiesClose()
            setPropertiesTrack(null)
          }}
          mediaFileId={propertiesTrack?.track.mediaFileId ?? null}
          title={propertiesTrack ? `${album.name} - ${propertiesTrack.track.title}` : undefined}
        />
      </div>
    </ShimmerLoader>
  )
}
