import { createFileRoute, Link, redirect, useNavigate } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useMemo } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Chip } from '@heroui/chip'
import { Image } from '@heroui/image'
import { ShimmerLoader } from '../../components/shared/ShimmerLoader'
import { audiobookTemplate, libraryTemplate } from '../../lib/template-data'
import { Breadcrumbs, BreadcrumbItem } from '@heroui/breadcrumbs'
import { Progress } from '@heroui/progress'
import { useDisclosure } from '@heroui/modal'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { addToast } from '@heroui/toast'
import { RouteError } from '../../components/RouteError'
import { sanitizeError, formatBytes, formatDuration } from '../../lib/format'
import {
  graphqlClient,
  AUDIOBOOK_WITH_CHAPTERS_QUERY,
  LIBRARY_QUERY,
  DELETE_AUDIOBOOK_MUTATION,
} from '../../lib/graphql'
import type {
  AudiobookWithChapters,
  AudiobookChapter,
  Library,
} from '../../lib/graphql'
import { DataTable, type DataTableColumn, type RowAction } from '../../components/data-table'
import {
  IconBook,
  IconHeadphones,
  IconSearch,
  IconRefresh,
  IconPlayerPlay,
  IconTrash,
  IconUser,
} from '@tabler/icons-react'
import { ChapterStatusChip, PlayPauseIndicator } from '../../components/shared'
import { usePlaybackContext } from '../../contexts/PlaybackContext'
import { useDataReactivity } from '../../hooks/useSubscription'

export const Route = createFileRoute('/audiobooks/$audiobookId')({
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
  component: AudiobookDetailPage,
  errorComponent: RouteError,
})

// Chapter table columns
const chapterColumns: DataTableColumn<AudiobookChapter>[] = [
  {
    key: 'chapterNumber',
    label: '#',
    width: 60,
    sortable: true,
    render: (ch) => (
      <span className="font-mono text-default-500">
        {String(ch.chapterNumber).padStart(2, '0')}
      </span>
    ),
  },
  {
    key: 'title',
    label: 'Title',
    sortable: true,
    render: (ch) => (
      <span className="font-medium">
        {ch.title || `Chapter ${ch.chapterNumber}`}
      </span>
    ),
  },
  {
    key: 'duration',
    label: 'Duration',
    width: 100,
    render: (ch) => (
      <span className="text-default-500 text-sm">
        {ch.durationSecs ? formatDuration(ch.durationSecs) : '-'}
      </span>
    ),
  },
  {
    key: 'status',
    label: 'Status',
    width: 140,
    sortable: true,
    render: (ch) => (
      <ChapterStatusChip
        mediaFileId={ch.mediaFileId}
        downloadProgress={ch.downloadProgress}
      />
    ),
  },
]

interface ChapterTableProps {
  chapters: AudiobookChapter[]
  audiobookId: string
  onPlay: (chapter: AudiobookChapter) => void
  onSearch: (chapter: AudiobookChapter) => void
}

function ChapterTable({ chapters, audiobookId, onPlay, onSearch }: ChapterTableProps) {
  // Get session and updatePlayback directly from context for reliable updates
  const { session, updatePlayback } = usePlaybackContext()

  // Handle pause directly using context
  const handlePause = useCallback(() => {
    updatePlayback({ isPlaying: false })
  }, [updatePlayback])

  // Compute playing state from session
  // Find chapter by matching mediaFileId since session doesn't have chapterId
  const currentlyPlayingChapterId = session?.audiobookId === audiobookId
    ? chapters.find(ch => ch.mediaFileId === session?.mediaFileId)?.id ?? null
    : null
  const isPlaying = session?.isPlaying ?? false

  // Row actions - computed fresh on each render to ensure playing state is always current
  const rowActions: RowAction<AudiobookChapter>[] = [
    // Playing indicator with pause on hover - shown for currently playing chapter
    {
      key: `playing-${currentlyPlayingChapterId || 'none'}`,
      label: 'Pause',
      icon: <PlayPauseIndicator size={16} isPlaying={isPlaying} colorClass="bg-success" />,
      color: 'default',
      inDropdown: false,
      isVisible: (ch) => ch.status === 'downloaded' && !!ch.mediaFileId && currentlyPlayingChapterId === ch.id && isPlaying,
      onAction: () => handlePause(),
    },
    // Play action - shown for all other chapters or when paused
    {
      key: `play-${currentlyPlayingChapterId || 'none'}-${isPlaying}`,
      label: 'Play',
      icon: <IconPlayerPlay size={16} />,
      color: 'success',
      inDropdown: false,
      isVisible: (ch) => ch.status === 'downloaded' && !!ch.mediaFileId && !(currentlyPlayingChapterId === ch.id && isPlaying),
      onAction: (ch) => onPlay(ch),
    },
    {
      key: 'search',
      label: 'Search',
      icon: <IconSearch size={16} />,
      color: 'default',
      inDropdown: false,
      isVisible: (ch) => ch.status === 'missing' || ch.status === 'wanted',
      onAction: (ch) => onSearch(ch),
    },
  ]

  // Create selection set for highlighting currently playing chapter
  const selectedKeys = useMemo(() => {
    if (currentlyPlayingChapterId) {
      return new Set([currentlyPlayingChapterId])
    }
    return new Set<string>()
  }, [currentlyPlayingChapterId])

  // Key that changes when playback state changes to force re-render
  const tableKey = `chapters-${currentlyPlayingChapterId || 'none'}-${isPlaying}`

  return (
    <DataTable
      key={tableKey}
      skeletonDelay={500}
      data={chapters}
      columns={chapterColumns}
      getRowKey={(ch) => ch.id}
      ariaLabel="Audiobook chapters"
      removeWrapper
      isCompact
      showItemCount={false}
      hideToolbar
      defaultSortColumn="chapterNumber"
      defaultSortDirection="asc"
      rowActions={rowActions}
      selectionMode={currentlyPlayingChapterId ? 'single' : 'none'}
      selectedKeys={selectedKeys}
    />
  )
}

function AudiobookDetailPage() {
  const { audiobookId } = Route.useParams()
  const navigate = useNavigate()
  const { startAudiobookPlayback } = usePlaybackContext()

  const [audiobookData, setAudiobookData] = useState<AudiobookWithChapters | null>(null)
  const [library, setLibrary] = useState<Library | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [isDeleting, setIsDeleting] = useState(false)
  const { isOpen: isDeleteOpen, onOpen: onDeleteOpen, onClose: onDeleteClose } = useDisclosure()

  // Update page title
  useEffect(() => {
    if (audiobookData) {
      document.title = `Librarian - ${audiobookData.audiobook.title}`
    }
    return () => {
      document.title = 'Librarian'
    }
  }, [audiobookData])

  // Fetch audiobook data
  const fetchAudiobook = useCallback(async () => {
    try {
      const result = await graphqlClient
        .query<{ audiobookWithChapters: AudiobookWithChapters | null }>(
          AUDIOBOOK_WITH_CHAPTERS_QUERY,
          { id: audiobookId }
        )
        .toPromise()

      if (result.error) {
        throw new Error(sanitizeError(result.error.message))
      }

      if (result.data?.audiobookWithChapters) {
        setAudiobookData(result.data.audiobookWithChapters)

        // Fetch library info
        const libResult = await graphqlClient
          .query<{
            Library: import('../../lib/graphql/generated/graphql').Library | null
          }>(LIBRARY_QUERY, {
            Id: result.data.audiobookWithChapters.audiobook.libraryId,
          })
          .toPromise()
        if (libResult.data?.Library) {
          setLibrary(libResult.data.Library)
        }
      } else {
        setError('Audiobook not found')
      }
    } catch (e) {
      setError(sanitizeError(e))
    } finally {
      setIsLoading(false)
    }
  }, [audiobookId])

  useEffect(() => {
    fetchAudiobook()
  }, [fetchAudiobook])

  // Keep data fresh with periodic updates and torrent completion events
  // This ensures download progress is updated in real-time
  useDataReactivity(fetchAudiobook, {
    onTorrentComplete: true,
    periodicInterval: 10000, // Refresh every 10 seconds to match backend sync interval
    onFocus: true,
  })

  // Handle delete
  const handleDelete = useCallback(async () => {
    if (!audiobookData) return
    setIsDeleting(true)
    try {
      const result = await graphqlClient
        .mutation<{ deleteAudiobook: { success: boolean; error?: string } }>(
          DELETE_AUDIOBOOK_MUTATION,
          { id: audiobookId }
        )
        .toPromise()

      if (result.data?.deleteAudiobook.success) {
        addToast({
          title: 'Audiobook deleted',
          description: `${audiobookData.audiobook.title} has been removed.`,
          color: 'success',
        })
        navigate({ to: '/libraries/$libraryId', params: { libraryId: audiobookData.audiobook.libraryId } })
      } else {
        addToast({
          title: 'Delete failed',
          description: result.data?.deleteAudiobook.error || 'Failed to delete audiobook',
          color: 'danger',
        })
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    } finally {
      setIsDeleting(false)
      onDeleteClose()
    }
  }, [audiobookData, audiobookId, navigate, onDeleteClose])

  // Navigate to hunt page for this audiobook
  const handleManualHunt = useCallback(() => {
    if (!audiobookData) return
    const searchQuery = audiobookData.audiobook.title
    navigate({
      to: '/hunt',
      search: { q: searchQuery, type: 'audiobooks' },
    })
  }, [audiobookData, navigate])

  // Handle play chapter - start playback with the audio player
  const handlePlayChapter = useCallback((chapter: AudiobookChapter) => {
    if (chapter.mediaFileId && audiobookData) {
      // Get all chapters that have media files for the queue
      const allChapters = audiobookData.chapters.filter(ch => ch.mediaFileId)
      startAudiobookPlayback(audiobookData.audiobook, chapter, allChapters)
    }
  }, [audiobookData, startAudiobookPlayback])

  // Note: Playing state and pause handling are now inside ChapterTable component directly from context

  // Navigate to hunt page for a specific chapter
  const handleSearchChapter = useCallback((chapter: AudiobookChapter) => {
    if (!audiobookData) return
    const searchQuery = `${audiobookData.audiobook.title} ${chapter.title || `Chapter ${chapter.chapterNumber}`}`
    navigate({
      to: '/hunt',
      search: { q: searchQuery, type: 'audiobooks' },
    })
  }, [audiobookData, navigate])

  // Show error state only after loading is complete
  if (!isLoading && (error || !audiobookData)) {
    return (
      <div className="container mx-auto p-4">
        <Card>
          <CardBody className="text-center py-12">
            <IconBook size={48} className="mx-auto mb-4 text-default-400" />
            <h2 className="text-xl font-semibold mb-2">Audiobook Not Found</h2>
            <p className="text-default-500 mb-4">{error || 'The audiobook could not be loaded.'}</p>
            <Button as={Link} to="/libraries" variant="flat">
              Back to Libraries
            </Button>
          </CardBody>
        </Card>
      </div>
    )
  }

  // Use template data during loading, real data when available
  const displayAudiobookData = audiobookData ?? audiobookTemplate
  const displayLibrary = library ?? libraryTemplate
  const { audiobook, chapters, author } = displayAudiobookData

  return (
    <ShimmerLoader loading={isLoading} delay={500} templateProps={{ audiobookData: audiobookTemplate }}>
      <div className="container mx-auto p-4  mb-20">
        {/* Breadcrumbs */}
        <Breadcrumbs className="mb-4">
          <BreadcrumbItem>
            <Link to="/libraries">Libraries</Link>
          </BreadcrumbItem>
          <BreadcrumbItem>
            <Link to="/libraries/$libraryId" params={{ libraryId: displayLibrary.Id }}>
              {displayLibrary.Name}
            </Link>
          </BreadcrumbItem>
          <BreadcrumbItem>{audiobook.title}</BreadcrumbItem>
        </Breadcrumbs>

        {/* Audiobook Header */}
        <div className="flex flex-col md:flex-row gap-6 mb-6">
          {/* Cover Art */}
          <div className="w-64 shrink-0">
            {audiobook.coverUrl ? (
              <Image
                src={audiobook.coverUrl}
                alt={audiobook.title}
                classNames={{
                  wrapper: 'w-64 h-64',
                  img: 'w-full h-full object-cover rounded-lg',
                }}
              />
            ) : (
              <div className="w-64 h-64 bg-content2 rounded-lg flex items-center justify-center">
                <IconBook size={64} className="text-default-400" />
              </div>
            )}
          </div>

          {/* Audiobook Info */}
          <div className="flex-1">
            <h1 className="text-3xl font-bold mb-1">{audiobook.title}</h1>
            {audiobook.subtitle && (
              <p className="text-lg text-default-500 mb-2">{audiobook.subtitle}</p>
            )}

            {/* Author */}
            {author && (
              <div className="flex items-center gap-2 text-default-600 mb-3">
                <IconUser size={16} />
                <span>by {author.name}</span>
              </div>
            )}

            {/* Tags */}
            <div className="flex flex-wrap items-center gap-2 mb-4">
              {audiobook.seriesName && (
                <Chip size="sm" variant="flat" color="secondary">
                  {audiobook.seriesName}
                </Chip>
              )}
              {audiobook.language && (
                <Chip size="sm" variant="flat">
                  {audiobook.language.toUpperCase()}
                </Chip>
              )}
              {audiobook.publisher && (
                <Chip size="sm" variant="flat">
                  {audiobook.publisher}
                </Chip>
              )}
            </div>

            {/* Description */}
            {audiobook.description && (
              <p className="text-default-600 mb-4 line-clamp-3">{audiobook.description}</p>
            )}

            {/* Narrators */}
            {audiobook.narrators && audiobook.narrators.length > 0 && (
              <div className="mb-4">
                <p className="text-sm text-default-400">
                  Narrated by: <span className="text-default-600">{audiobook.narrators.join(', ')}</span>
                </p>
              </div>
            )}

            {/* Completion Progress */}
            <div className="mb-4">
              <div className="flex items-center justify-between text-sm mb-1">
                <span className="text-default-500">
                  {displayAudiobookData.chaptersWithFiles} of {displayAudiobookData.chapterCount} chapters
                </span>
                <span className="font-medium">
                  {displayAudiobookData.completionPercent.toFixed(0)}%
                </span>
              </div>
              <Progress
                aria-label="Audiobook completion"
                value={displayAudiobookData.completionPercent}
                color={displayAudiobookData.completionPercent === 100 ? 'success' : 'primary'}
                size="sm"
              />
              {displayAudiobookData.missingChapters > 0 && (
                <p className="text-sm text-warning mt-1">
                  {displayAudiobookData.missingChapters} chapter{displayAudiobookData.missingChapters !== 1 ? 's' : ''} missing
                </p>
              )}
            </div>

            {/* Stats */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
              <div>
                <p className="text-xs text-default-400">Chapters</p>
                <p className="text-lg font-semibold">{displayAudiobookData.chapterCount}</p>
              </div>
              <div>
                <p className="text-xs text-default-400">Duration</p>
                <p className="text-lg font-semibold">
                  {audiobook.durationSecs ? formatDuration(audiobook.durationSecs) : '-'}
                </p>
              </div>
              <div>
                <p className="text-xs text-default-400">Size</p>
                <p className="text-lg font-semibold">
                  {audiobook.sizeBytes ? formatBytes(audiobook.sizeBytes) : '-'}
                </p>
              </div>
              <div>
                <p className="text-xs text-default-400">ISBN</p>
                <p className="text-lg font-semibold">
                  {audiobook.isbn || '-'}
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
                Hunt for Audiobook
              </Button>
              <Button
                variant="flat"
                startContent={<IconRefresh size={16} />}
                onPress={fetchAudiobook}
              >
                Refresh
              </Button>
              <Button
                variant="flat"
                color="danger"
                startContent={<IconTrash size={16} />}
                onPress={onDeleteOpen}
              >
                Delete
              </Button>
            </div>
          </div>
        </div>

        {/* Chapter List */}
        <Card>
          <CardBody className="p-0">
            <div className="p-4 border-b border-default-200">
              <h2 className="text-lg font-semibold flex items-center gap-2">
                <IconHeadphones size={20} className="text-orange-400" />
                Chapters
              </h2>
            </div>

            {chapters.length === 0 ? (
              <div className="p-8 text-center">
                <IconHeadphones size={48} className="mx-auto mb-4 text-default-400" />
                <h3 className="text-lg font-semibold mb-2">No Chapters</h3>
                <p className="text-default-500 mb-4">
                  Chapter information hasn't been fetched yet.
                </p>
                <Button variant="flat" onPress={fetchAudiobook}>
                  Refresh Audiobook
                </Button>
              </div>
            ) : (
              <ChapterTable
                chapters={chapters}
                audiobookId={displayAudiobookData.audiobook.id}
                onPlay={handlePlayChapter}
                onSearch={handleSearchChapter}
              />
            )}
          </CardBody>
        </Card>

        {/* Delete Confirmation Modal */}
        {audiobookData && (
          <Modal isOpen={isDeleteOpen} onClose={onDeleteClose}>
            <ModalContent>
              <ModalHeader>Delete Audiobook</ModalHeader>
              <ModalBody>
                <p>
                  Are you sure you want to delete <strong>{audiobookData.audiobook.title}</strong>?
                </p>
                <p className="text-sm text-default-500 mt-2">
                  This will remove the audiobook from the library. Associated files will not be deleted.
                </p>
              </ModalBody>
              <ModalFooter>
                <Button variant="flat" onPress={onDeleteClose}>
                  Cancel
                </Button>
                <Button
                  color="danger"
                  onPress={handleDelete}
                  isLoading={isDeleting}
                >
                  Delete
                </Button>
              </ModalFooter>
            </ModalContent>
          </Modal>
        )}
      </div>
    </ShimmerLoader>
  )
}
