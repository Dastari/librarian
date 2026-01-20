import { createFileRoute, Link, redirect, useNavigate } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Chip } from '@heroui/chip'
import { Image } from '@heroui/image'
import { Skeleton } from '@heroui/skeleton'
import { Breadcrumbs, BreadcrumbItem } from '@heroui/breadcrumbs'
import { Progress } from '@heroui/progress'
import { Tooltip } from '@heroui/tooltip'
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
import { DataTable, type DataTableColumn } from '../../components/data-table'
import {
  IconDisc,
  IconMusic,
  IconCheck,
  IconX,
  IconSearch,
  IconRefresh,
} from '@tabler/icons-react'

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

// Track table columns
const trackColumns: DataTableColumn<TrackWithStatus>[] = [
  {
    key: 'trackNumber',
    label: '#',
    width: 50,
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
    key: 'status',
    label: 'Status',
    width: 100,
    render: (t) => (
      <div className="flex items-center gap-1">
        {t.hasFile ? (
          <Chip size="sm" color="success" variant="flat" startContent={<IconCheck size={12} />}>
            Downloaded
          </Chip>
        ) : (
          <Chip size="sm" color="warning" variant="flat" startContent={<IconX size={12} />}>
            Wanted
          </Chip>
        )}
      </div>
    ),
  },
  {
    key: 'file',
    label: 'File',
    width: 200,
    render: (t) => (
      <div className="flex flex-col text-xs text-default-400 truncate max-w-[200px]">
        {t.filePath ? (
          <Tooltip content={t.filePath}>
            <span className="truncate">{t.filePath.split('/').pop()}</span>
          </Tooltip>
        ) : (
          <span>-</span>
        )}
        {t.fileSize && <span>{formatBytes(t.fileSize)}</span>}
      </div>
    ),
  },
]

function AlbumDetailPage() {
  const { albumId } = Route.useParams()
  const navigate = useNavigate()

  const [albumData, setAlbumData] = useState<AlbumWithTracks | null>(null)
  const [library, setLibrary] = useState<Library | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

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

  // Navigate to hunt page for this album
  const handleManualHunt = () => {
    if (!albumData) return
    const searchQuery = `${albumData.album.name}`
    navigate({
      to: '/hunt',
      search: { q: searchQuery, type: 'music' },
    })
  }

  if (isLoading) {
    return (
      <div className="container mx-auto p-4 max-w-6xl">
        <Skeleton className="h-8 w-64 mb-4" />
        <div className="flex gap-6">
          <Skeleton className="w-64 h-64 rounded-lg" />
          <div className="flex-1 space-y-4">
            <Skeleton className="h-8 w-96" />
            <Skeleton className="h-4 w-48" />
            <Skeleton className="h-4 w-64" />
          </div>
        </div>
      </div>
    )
  }

  if (error || !albumData) {
    return (
      <div className="container mx-auto p-4 max-w-6xl">
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

  const { album, tracks } = albumData

  return (
    <div className="container mx-auto p-4 max-w-6xl">
      {/* Breadcrumbs */}
      <Breadcrumbs className="mb-4">
        <BreadcrumbItem>
          <Link to="/libraries">Libraries</Link>
        </BreadcrumbItem>
        {library && (
          <BreadcrumbItem>
            <Link to="/libraries/$libraryId" params={{ libraryId: library.id }}>{library.name}</Link>
          </BreadcrumbItem>
        )}
        <BreadcrumbItem>{album.name}</BreadcrumbItem>
      </Breadcrumbs>

      {/* Album Header */}
      <div className="flex flex-col md:flex-row gap-6 mb-6">
        {/* Cover Art */}
        <div className="w-64 shrink-0">
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
        </div>

        {/* Album Info */}
        <div className="flex-1">
          <h1 className="text-3xl font-bold mb-2">{album.name}</h1>

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
                {albumData.tracksWithFiles} of {albumData.trackCount} tracks
              </span>
              <span className="font-medium">
                {albumData.completionPercent.toFixed(0)}%
              </span>
            </div>
            <Progress
              value={albumData.completionPercent}
              color={albumData.completionPercent === 100 ? 'success' : 'primary'}
              size="sm"
            />
            {albumData.missingTracks > 0 && (
              <p className="text-sm text-warning mt-1">
                {albumData.missingTracks} track{albumData.missingTracks !== 1 ? 's' : ''} wanted
              </p>
            )}
          </div>

          {/* Stats */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
            <div>
              <p className="text-xs text-default-400">Tracks</p>
              <p className="text-lg font-semibold">{albumData.trackCount}</p>
            </div>
            <div>
              <p className="text-xs text-default-400">Discs</p>
              <p className="text-lg font-semibold">{album.discCount || 1}</p>
            </div>
            <div>
              <p className="text-xs text-default-400">Duration</p>
              <p className="text-lg font-semibold">
                {album.totalDurationSecs
                  ? formatDuration(album.totalDurationSecs)
                  : '-'}
              </p>
            </div>
            <div>
              <p className="text-xs text-default-400">Size</p>
              <p className="text-lg font-semibold">
                {album.sizeBytes ? formatBytes(album.sizeBytes) : '-'}
              </p>
            </div>
          </div>

          {/* Actions */}
          <div className="flex flex-wrap gap-2">
            <Button
              color="primary"
              startContent={<IconSearch size={16} />}
              onPress={handleManualHunt}
            >
              Hunt for Album
            </Button>
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

      {/* Track List */}
      <Card>
        <CardBody className="p-0">
          <div className="p-4 border-b border-default-200">
            <h2 className="text-lg font-semibold flex items-center gap-2">
              <IconMusic size={20} className="text-green-400" />
              Tracks
            </h2>
          </div>

          {tracks.length === 0 ? (
            <div className="p-8 text-center">
              <IconMusic size={48} className="mx-auto mb-4 text-default-400" />
              <h3 className="text-lg font-semibold mb-2">No Tracks</h3>
              <p className="text-default-500 mb-4">
                Track information hasn't been fetched yet.
              </p>
              <Button variant="flat" onPress={fetchAlbum}>
                Refresh Album
              </Button>
            </div>
          ) : (
            <DataTable
              data={tracks}
              columns={trackColumns}
              getRowKey={(t) => t.track.id}
              defaultSortColumn="trackNumber"
              defaultSortDirection="asc"
              searchPlaceholder="Search tracks..."
              searchFn={(item, term) => {
                const searchLower = term.toLowerCase()
                return (
                  item.track.title.toLowerCase().includes(searchLower) ||
                  (item.track.artistName?.toLowerCase().includes(searchLower) ?? false)
                )
              }}
            />
          )}
        </CardBody>
      </Card>
    </div>
  )
}
