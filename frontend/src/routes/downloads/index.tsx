import { createFileRoute, redirect } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import { getAccessToken } from '../../lib/auth'
import { useDisclosure } from '@heroui/modal'
import { addToast } from '@heroui/toast'
import { Tabs, Tab } from '@heroui/tabs'
import { Chip } from '@heroui/chip'
import { Card, CardBody } from '@heroui/card'
import { Button } from '@heroui/button'
import { Progress } from '@heroui/progress'
import { Spinner } from '@heroui/spinner'
import { IconDownload, IconServer, IconPlayerPause, IconPlayerPlay, IconTrash, IconRefresh } from '@tabler/icons-react'
import {
  graphqlClient,
  TORRENTS_QUERY,
  ADD_TORRENT_MUTATION,
  PAUSE_TORRENT_MUTATION,
  RESUME_TORRENT_MUTATION,
  REMOVE_TORRENT_MUTATION,
  ORGANIZE_TORRENT_MUTATION,
  PROCESS_SOURCE_MUTATION,
  REMATCH_SOURCE_MUTATION,
  TORRENT_PROGRESS_SUBSCRIPTION,
  TORRENT_ADDED_SUBSCRIPTION,
  TORRENT_REMOVED_SUBSCRIPTION,
  type Torrent,
  type TorrentProgress,
  type AddTorrentResult,
  type TorrentActionResult,
  type OrganizeTorrentResult,
  type ProcessSourceResult,
  type RematchSourceResult,
} from '../../lib/graphql'
import { TorrentTable, AddTorrentModal, TorrentInfoModal, LinkToLibraryModal } from '../../components/downloads'
import { sanitizeError, formatBytes } from '../../lib/format'
import { RouteError } from '../../components/RouteError'

// Usenet download type
interface UsenetDownload {
  id: string
  name: string
  state: string
  progress: number
  size: number | null
  downloaded: number
  downloadSpeed: number
  etaSeconds: number | null
  errorMessage: string | null
}

// Usenet GraphQL queries
const USENET_DOWNLOADS_QUERY = `
  query UsenetDownloads {
    usenetDownloads {
      id
      name
      state
      progress
      size
      downloaded
      downloadSpeed
      etaSeconds
      errorMessage
    }
  }
`

const PAUSE_USENET_MUTATION = `
  mutation PauseUsenet($id: ID!) {
    pauseUsenetDownload(id: $id) {
      success
      error
    }
  }
`

const RESUME_USENET_MUTATION = `
  mutation ResumeUsenet($id: ID!) {
    resumeUsenetDownload(id: $id) {
      success
      error
    }
  }
`

const REMOVE_USENET_MUTATION = `
  mutation RemoveUsenet($id: ID!, $deleteFiles: Boolean) {
    removeUsenetDownload(id: $id, deleteFiles: $deleteFiles) {
      success
      error
    }
  }
`

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
  const [activeTab, setActiveTab] = useState<'torrents' | 'usenet'>('torrents')
  const [torrents, setTorrents] = useState<Torrent[]>([])
  const [usenetDownloads, setUsenetDownloads] = useState<UsenetDownload[]>([])
  const [isAdding, setIsAdding] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const [isUsenetLoading, setIsUsenetLoading] = useState(true)
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
        // Silently ignore auth errors - they can happen during login race conditions
        const isAuthError = result.error.message?.toLowerCase().includes('authentication');
        if (!isAuthError) {
          addToast({
            title: 'Error',
            description: sanitizeError(result.error),
            color: 'danger',
          })
        }
      }
    } catch (e) {
      // Silently ignore auth errors
      const errorMsg = e instanceof Error ? e.message : String(e);
      if (!errorMsg.toLowerCase().includes('authentication')) {
        addToast({
          title: 'Error',
          description: sanitizeError(e),
          color: 'danger',
        })
      }
    } finally {
      setIsLoading(false)
    }
  }, [])

  const fetchUsenetDownloads = useCallback(async () => {
    try {
      const result = await graphqlClient.query<{ usenetDownloads: UsenetDownload[] }>(USENET_DOWNLOADS_QUERY, {}).toPromise()
      if (result.data?.usenetDownloads) {
        setUsenetDownloads(result.data.usenetDownloads)
      }
    } catch (e) {
      // Silently ignore auth errors
      const errorMsg = e instanceof Error ? e.message : String(e);
      if (!errorMsg.toLowerCase().includes('authentication')) {
        console.error('Failed to fetch usenet downloads:', e)
      }
    } finally {
      setIsUsenetLoading(false)
    }
  }, [])

  // Fetch torrents and usenet downloads, subscribe to updates
  useEffect(() => {
    fetchTorrents()
    fetchUsenetDownloads()

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
  }, [fetchTorrents, fetchUsenetDownloads])

  // Usenet action handlers
  const handlePauseUsenet = async (id: string) => {
    const result = await graphqlClient
      .mutation<{ pauseUsenetDownload: { success: boolean; error?: string } }>(PAUSE_USENET_MUTATION, { id })
      .toPromise()
    if (result.data?.pauseUsenetDownload.success) {
      setUsenetDownloads((prev) =>
        prev.map((d) => (d.id === id ? { ...d, state: 'paused' } : d))
      )
    }
  }

  const handleResumeUsenet = async (id: string) => {
    const result = await graphqlClient
      .mutation<{ resumeUsenetDownload: { success: boolean; error?: string } }>(RESUME_USENET_MUTATION, { id })
      .toPromise()
    if (result.data?.resumeUsenetDownload.success) {
      setUsenetDownloads((prev) =>
        prev.map((d) => (d.id === id ? { ...d, state: 'downloading' } : d))
      )
    }
  }

  const handleRemoveUsenet = async (id: string) => {
    const result = await graphqlClient
      .mutation<{ removeUsenetDownload: { success: boolean; error?: string } }>(REMOVE_USENET_MUTATION, { id, deleteFiles: false })
      .toPromise()
    if (result.data?.removeUsenetDownload.success) {
      setUsenetDownloads((prev) => prev.filter((d) => d.id !== id))
      addToast({
        title: 'Usenet Download Removed',
        description: 'The download has been removed.',
        color: 'success',
      })
    }
  }

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

      // Get auth token from cookie storage
      const authToken = getAccessToken() || ''

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

  // Process pending file matches (copy files to library)
  const handleProcess = async (torrent: Torrent) => {
    const result = await graphqlClient
      .mutation<{ processSource: ProcessSourceResult }>(PROCESS_SOURCE_MUTATION, {
        sourceType: 'torrent',
        sourceId: torrent.infoHash,
      })
      .toPromise()

    if (result.data?.processSource) {
      const proc = result.data.processSource
      if (proc.success) {
        addToast({
          title: 'Files Processed',
          description: `Copied ${proc.filesProcessed} file(s) to library${proc.filesFailed > 0 ? `, ${proc.filesFailed} failed` : ''}`,
          color: 'success',
        })
      } else {
        addToast({
          title: 'Processing Failed',
          description: proc.error || proc.messages[0] || 'Failed to process files',
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

  // Re-match files against library items
  const handleRematch = async (torrent: Torrent) => {
    const result = await graphqlClient
      .mutation<{ rematchSource: RematchSourceResult }>(REMATCH_SOURCE_MUTATION, {
        sourceType: 'torrent',
        sourceId: torrent.infoHash,
        libraryId: null, // Match against all libraries
      })
      .toPromise()

    if (result.data?.rematchSource) {
      const match = result.data.rematchSource
      if (match.success) {
        addToast({
          title: 'Files Rematched',
          description: `Found ${match.matchCount} match(es)`,
          color: 'success',
        })
      } else {
        addToast({
          title: 'Rematch Failed',
          description: match.error || 'Failed to rematch files',
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
          <p className="text-default-500">Manage your torrent and usenet downloads</p>
        </div>
      </div>

      {/* Tabs for Torrents and Usenet */}
      <Tabs
        selectedKey={activeTab}
        onSelectionChange={(key) => setActiveTab(key as 'torrents' | 'usenet')}
        className="mb-4"
        classNames={{
          tabList: 'gap-4',
        }}
      >
        <Tab
          key="torrents"
          title={
            <div className="flex items-center gap-2">
              <IconDownload size={18} />
              <span>Torrents</span>
              {torrents.length > 0 && (
                <Chip size="sm" variant="flat">
                  {torrents.length}
                </Chip>
              )}
            </div>
          }
        />
        <Tab
          key="usenet"
          title={
            <div className="flex items-center gap-2">
              <IconServer size={18} />
              <span>Usenet</span>
              {usenetDownloads.length > 0 && (
                <Chip size="sm" variant="flat">
                  {usenetDownloads.length}
                </Chip>
              )}
            </div>
          }
        />
      </Tabs>

      {/* Tab Content */}
      {activeTab === 'torrents' ? (
        <>
          {/* Torrents table - skeleton loading handled by DataTable */}
          <TorrentTable
            torrents={torrents}
            isLoading={isLoading}
            onPause={handlePause}
            onResume={handleResume}
            onRemove={handleRemove}
            onInfo={handleInfo}
            onOrganize={handleOrganize}
            onProcess={handleProcess}
            onRematch={handleRematch}
            onLinkToLibrary={(torrent) => {
              setTorrentToLink(torrent)
              onLinkOpen()
            }}
            onBulkPause={handleBulkPause}
            onBulkResume={handleBulkResume}
            onBulkRemove={handleBulkRemove}
            onAddClick={onOpen}
          />
        </>
      ) : (
        <>
          {/* Usenet Downloads */}
          <div className="flex justify-end mb-4">
            <Button
              size="sm"
              variant="flat"
              startContent={<IconRefresh size={16} />}
              onPress={fetchUsenetDownloads}
              isLoading={isUsenetLoading}
            >
              Refresh
            </Button>
          </div>

          {isUsenetLoading ? (
            <div className="flex justify-center py-12">
              <Spinner size="lg" />
            </div>
          ) : usenetDownloads.length === 0 ? (
            <Card>
              <CardBody className="flex flex-col items-center justify-center py-12 text-center">
                <IconServer size={48} className="text-default-300 mb-4" />
                <p className="text-default-500">No usenet downloads</p>
                <p className="text-default-400 text-sm mt-1">
                  Add NZB files from the Hunt page or configure Newznab indexers
                </p>
              </CardBody>
            </Card>
          ) : (
            <div className="space-y-3">
              {usenetDownloads.map((download) => (
                <Card key={download.id}>
                  <CardBody className="flex flex-row items-center gap-4">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-medium truncate">{download.name}</span>
                        <Chip
                          size="sm"
                          variant="flat"
                          color={
                            download.state === 'completed' ? 'success' :
                            download.state === 'downloading' ? 'primary' :
                            download.state === 'paused' ? 'warning' :
                            download.state === 'failed' ? 'danger' : 'default'
                          }
                        >
                          {download.state}
                        </Chip>
                      </div>
                      <div className="flex items-center gap-4 text-sm text-default-500">
                        <span>{download.size ? formatBytes(download.size) : 'Unknown size'}</span>
                        {download.downloadSpeed > 0 && (
                          <span>{formatBytes(download.downloadSpeed)}/s</span>
                        )}
                        {download.etaSeconds && download.etaSeconds > 0 && (
                          <span>ETA: {Math.floor(download.etaSeconds / 60)}m</span>
                        )}
                      </div>
                      {download.state === 'downloading' && (
                        <Progress
                          value={download.progress}
                          className="mt-2"
                          size="sm"
                          color="primary"
                        />
                      )}
                      {download.errorMessage && (
                        <p className="text-danger text-sm mt-1">{download.errorMessage}</p>
                      )}
                    </div>
                    <div className="flex gap-2">
                      {download.state === 'downloading' ? (
                        <Button
                          isIconOnly
                          size="sm"
                          variant="flat"
                          onPress={() => handlePauseUsenet(download.id)}
                          aria-label="Pause download"
                        >
                          <IconPlayerPause size={16} />
                        </Button>
                      ) : download.state === 'paused' ? (
                        <Button
                          isIconOnly
                          size="sm"
                          variant="flat"
                          color="primary"
                          onPress={() => handleResumeUsenet(download.id)}
                          aria-label="Resume download"
                        >
                          <IconPlayerPlay size={16} />
                        </Button>
                      ) : null}
                      <Button
                        isIconOnly
                        size="sm"
                        variant="flat"
                        color="danger"
                        onPress={() => handleRemoveUsenet(download.id)}
                        aria-label="Remove download"
                      >
                        <IconTrash size={16} />
                      </Button>
                    </div>
                  </CardBody>
                </Card>
              ))}
            </div>
          )}
        </>
      )}

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
