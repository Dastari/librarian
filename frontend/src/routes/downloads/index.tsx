import { createFileRoute, redirect } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import { useDisclosure } from '@heroui/modal'
import { addToast } from '@heroui/toast'
import {
  graphqlClient,
  TORRENTS_QUERY,
  ADD_TORRENT_MUTATION,
  PAUSE_TORRENT_MUTATION,
  RESUME_TORRENT_MUTATION,
  REMOVE_TORRENT_MUTATION,
  ORGANIZE_TORRENT_MUTATION,
  TORRENT_PROGRESS_SUBSCRIPTION,
  TORRENT_ADDED_SUBSCRIPTION,
  TORRENT_REMOVED_SUBSCRIPTION,
  type Torrent,
  type TorrentProgress,
  type AddTorrentResult,
  type TorrentActionResult,
  type OrganizeTorrentResult,
} from '../../lib/graphql'
import { TorrentTable, AddTorrentModal, TorrentInfoModal, LinkToLibraryModal } from '../../components/downloads'
import { sanitizeError } from '../../lib/format'
import { RouteError } from '../../components/RouteError'

export const Route = createFileRoute('/downloads/')({
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
  component: DownloadsPage,
  errorComponent: RouteError,
})

function DownloadsPage() {
  const [torrents, setTorrents] = useState<Torrent[]>([])
  const [isAdding, setIsAdding] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const { isOpen, onOpen, onClose } = useDisclosure()
  const { isOpen: isInfoOpen, onOpen: onInfoOpen, onClose: onInfoClose } = useDisclosure()
  const { isOpen: isLinkOpen, onOpen: onLinkOpen, onClose: onLinkClose } = useDisclosure()
  const [selectedTorrentId, setSelectedTorrentId] = useState<number | null>(null)
  const [torrentToLink, setTorrentToLink] = useState<Torrent | null>(null)

  const fetchTorrents = useCallback(async () => {
    try {
      const result = await graphqlClient.query<{ torrents: Torrent[] }>(TORRENTS_QUERY, {}).toPromise()
      if (result.data?.torrents) {
        setTorrents(result.data.torrents)
      }
      if (result.error) {
        addToast({
          title: 'Error',
          description: sanitizeError(result.error),
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
      setIsLoading(false)
    }
  }, [])

  // Fetch torrents and subscribe to updates
  useEffect(() => {
    fetchTorrents()

    // Apollo subscriptions return Observables
    const progressSub = graphqlClient.subscription<{ torrentProgress: TorrentProgress }>(
      TORRENT_PROGRESS_SUBSCRIPTION,
      {}
    ).subscribe({
      next: (result) => {
        if (result.data?.torrentProgress) {
          const p = result.data.torrentProgress
          setTorrents((prev) =>
            prev.map((t) =>
              t.id === p.id
                ? {
                  ...t,
                  progress: p.progress,
                  downloadSpeed: p.downloadSpeed,
                  uploadSpeed: p.uploadSpeed,
                  peers: p.peers,
                  state: p.state,
                }
                : t
            )
          )
        }
      },
    })

    const addedSub = graphqlClient.subscription(TORRENT_ADDED_SUBSCRIPTION, {}).subscribe({
      next: () => fetchTorrents(),
    })

    const removedSub = graphqlClient.subscription<{ torrentRemoved: { id: number } }>(
      TORRENT_REMOVED_SUBSCRIPTION,
      {}
    ).subscribe({
      next: (result) => {
        if (result.data?.torrentRemoved) {
          setTorrents((prev) => prev.filter((t) => t.id !== result.data!.torrentRemoved.id))
        }
      },
    })

    return () => {
      progressSub.unsubscribe()
      addedSub.unsubscribe()
      removedSub.unsubscribe()
    }
  }, [fetchTorrents])

  // Add torrent handlers
  const handleAddMagnet = async (magnet: string) => {
    setIsAdding(true)
    try {
      const result = await graphqlClient
        .mutation<{ addTorrent: AddTorrentResult }>(ADD_TORRENT_MUTATION, {
          input: { magnet },
        })
        .toPromise()
      if (result.data?.addTorrent.success && result.data.addTorrent.torrent) {
        setTorrents((prev) => [result.data!.addTorrent.torrent!, ...prev])
        addToast({
          title: 'Torrent Added',
          description: `Started downloading: ${result.data.addTorrent.torrent.name}`,
          color: 'success',
        })
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(result.data?.addTorrent.error || result.error?.message || 'Failed'),
          color: 'danger',
        })
      }
    } catch {
      addToast({
        title: 'Error',
        description: 'Failed to add torrent',
        color: 'danger',
      })
    } finally {
      setIsAdding(false)
    }
  }

  const handleAddUrl = async (url: string) => {
    setIsAdding(true)
    try {
      const result = await graphqlClient
        .mutation<{ addTorrent: AddTorrentResult }>(ADD_TORRENT_MUTATION, {
          input: { url },
        })
        .toPromise()
      if (result.data?.addTorrent.success && result.data.addTorrent.torrent) {
        setTorrents((prev) => [result.data!.addTorrent.torrent!, ...prev])
        addToast({
          title: 'Torrent Added',
          description: `Started downloading: ${result.data.addTorrent.torrent.name}`,
          color: 'success',
        })
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(result.data?.addTorrent.error || result.error?.message || 'Failed'),
          color: 'danger',
        })
      }
    } catch {
      addToast({
        title: 'Error',
        description: 'Failed to add torrent',
        color: 'danger',
      })
    } finally {
      setIsAdding(false)
    }
  }

  const handleAddFile = async (file: File) => {
    setIsAdding(true)

    try {
      const formData = new FormData()
      formData.append('file', file)

      const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001'

      let authToken = ''
      try {
        const supabaseUrl = import.meta.env.VITE_SUPABASE_URL || 'http://localhost:54321'
        const projectId = new URL(supabaseUrl).hostname.split('.')[0]
        const storageKey = `sb-${projectId}-auth-token`
        const stored = localStorage.getItem(storageKey)
        if (stored) {
          const session = JSON.parse(stored)
          authToken = session?.access_token || ''
        }
      } catch {
        /* ignore */
      }

      const response = await fetch(`${API_URL}/api/torrents/upload`, {
        method: 'POST',
        headers: authToken ? { Authorization: `Bearer ${authToken}` } : {},
        body: formData,
      })

      const data = await response.json()

      if (data.success && data.torrent) {
        fetchTorrents()
        addToast({
          title: 'Torrent Added',
          description: `Started downloading: ${data.torrent.name}`,
          color: 'success',
        })
      } else {
        addToast({
          title: 'Error',
          description: data.error || 'Failed to upload torrent file',
          color: 'danger',
        })
      }
    } catch (e) {
      addToast({
        title: 'Error',
        description: 'Failed to upload torrent file',
        color: 'danger',
      })
      console.error(e)
    } finally {
      setIsAdding(false)
    }
  }

  // Single torrent actions
  const handlePause = async (id: number) => {
    const result = await graphqlClient
      .mutation<{ pauseTorrent: TorrentActionResult }>(PAUSE_TORRENT_MUTATION, { id })
      .toPromise()
    if (result.data?.pauseTorrent.success) {
      setTorrents((prev) =>
        prev.map((t) => (t.id === id ? { ...t, state: 'PAUSED' as const } : t))
      )
    }
  }

  const handleResume = async (id: number) => {
    const result = await graphqlClient
      .mutation<{ resumeTorrent: TorrentActionResult }>(RESUME_TORRENT_MUTATION, { id })
      .toPromise()
    if (result.data?.resumeTorrent.success) {
      setTorrents((prev) =>
        prev.map((t) => (t.id === id ? { ...t, state: 'DOWNLOADING' as const } : t))
      )
    }
  }

  const handleRemove = async (id: number) => {
    const result = await graphqlClient
      .mutation<{ removeTorrent: TorrentActionResult }>(REMOVE_TORRENT_MUTATION, {
        id,
        deleteFiles: false,
      })
      .toPromise()
    if (result.data?.removeTorrent.success) {
      setTorrents((prev) => prev.filter((t) => t.id !== id))
      addToast({
        title: 'Torrent Removed',
        description: 'The torrent has been removed.',
        color: 'success',
      })
    }
  }

  const handleInfo = (id: number) => {
    setSelectedTorrentId(id)
    onInfoOpen()
  }

  const handleOrganize = async (id: number) => {
    const result = await graphqlClient
      .mutation<{ organizeTorrent: OrganizeTorrentResult }>(ORGANIZE_TORRENT_MUTATION, {
        id,
        libraryId: null, // Will use first TV library
      })
      .toPromise()

    if (result.data?.organizeTorrent) {
      const org = result.data.organizeTorrent
      if (org.success) {
        addToast({
          title: 'Files Organized',
          description: `Organized ${org.organizedCount} file(s)${org.failedCount > 0 ? `, ${org.failedCount} failed` : ''}`,
          color: 'success',
        })
        // Show detailed messages if any
        if (org.messages.length > 0) {
          console.log('Organize messages:', org.messages)
        }
      } else {
        addToast({
          title: 'Organization Failed',
          description: org.messages[0] || 'Failed to organize files',
          color: 'danger',
        })
      }
    } else if (result.error) {
      addToast({
        title: 'Error',
        description: sanitizeError(result.error),
        color: 'danger',
      })
    }
  }

  // Bulk actions
  const handleBulkPause = async (ids: number[]) => {
    let successCount = 0
    for (const id of ids) {
      const result = await graphqlClient
        .mutation<{ pauseTorrent: TorrentActionResult }>(PAUSE_TORRENT_MUTATION, { id })
        .toPromise()
      if (result.data?.pauseTorrent.success) {
        successCount++
        setTorrents((prev) =>
          prev.map((t) => (t.id === id ? { ...t, state: 'PAUSED' as const } : t))
        )
      }
    }
    addToast({
      title: 'Paused Torrents',
      description: `Paused ${successCount} of ${ids.length} torrent(s)`,
      color: 'success',
    })
  }

  const handleBulkResume = async (ids: number[]) => {
    let successCount = 0
    for (const id of ids) {
      const result = await graphqlClient
        .mutation<{ resumeTorrent: TorrentActionResult }>(RESUME_TORRENT_MUTATION, { id })
        .toPromise()
      if (result.data?.resumeTorrent.success) {
        successCount++
        setTorrents((prev) =>
          prev.map((t) => (t.id === id ? { ...t, state: 'DOWNLOADING' as const } : t))
        )
      }
    }
    addToast({
      title: 'Resumed Torrents',
      description: `Resumed ${successCount} of ${ids.length} torrent(s)`,
      color: 'success',
    })
  }

  const handleBulkRemove = async (ids: number[]) => {
    let successCount = 0
    for (const id of ids) {
      const result = await graphqlClient
        .mutation<{ removeTorrent: TorrentActionResult }>(REMOVE_TORRENT_MUTATION, {
          id,
          deleteFiles: false,
        })
        .toPromise()
      if (result.data?.removeTorrent.success) {
        successCount++
        setTorrents((prev) => prev.filter((t) => t.id !== id))
      }
    }
    addToast({
      title: 'Removed Torrents',
      description: `Removed ${successCount} of ${ids.length} torrent(s)`,
      color: 'success',
    })
  }

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 min-w-0 grow flex flex-col ">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Downloads</h1>
          <p className="text-default-500">Manage your torrent downloads</p>
        </div>
      </div>

      {/* Torrents table - skeleton loading handled by DataTable */}
      <TorrentTable
        torrents={torrents}
        isLoading={isLoading}
        onPause={handlePause}
        onResume={handleResume}
        onRemove={handleRemove}
        onInfo={handleInfo}
        onOrganize={handleOrganize}
        onLinkToLibrary={(torrent) => {
          setTorrentToLink(torrent)
          onLinkOpen()
        }}
        onBulkPause={handleBulkPause}
        onBulkResume={handleBulkResume}
        onBulkRemove={handleBulkRemove}
        onAddClick={onOpen}
      />

      {/* Add Torrent Modal */}
      <AddTorrentModal
        isOpen={isOpen}
        onClose={onClose}
        onAddMagnet={handleAddMagnet}
        onAddUrl={handleAddUrl}
        onAddFile={handleAddFile}
        isLoading={isAdding}
      />

      {/* Torrent Info Modal */}
      <TorrentInfoModal
        torrentId={selectedTorrentId}
        isOpen={isInfoOpen}
        onClose={onInfoClose}
      />

      {/* Link to Library Modal */}
      <LinkToLibraryModal
        isOpen={isLinkOpen}
        onClose={onLinkClose}
        torrent={torrentToLink}
        onLinked={fetchTorrents}
      />
    </div>
  )
}
