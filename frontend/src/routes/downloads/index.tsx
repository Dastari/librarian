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
  DOWNLOADS_TORRENTS_QUERY,
  ADD_TORRENT_MUTATION,
  PAUSE_TORRENT_BY_INFO_HASH_MUTATION,
  RESUME_TORRENT_BY_INFO_HASH_MUTATION,
  REMOVE_TORRENT_BY_INFO_HASH_MUTATION,
  ORGANIZE_TORRENT_MUTATION,
  PROCESS_SOURCE_MUTATION,
  REMATCH_SOURCE_MUTATION,
  TorrentChangedDocument,
  type DownloadsTorrentRow,
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
  const [torrents, setTorrents] = useState<DownloadsTorrentRow[]>([])
  const [usenetDownloads, setUsenetDownloads] = useState<UsenetDownload[]>([])
  const [isAdding, setIsAdding] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const [isUsenetLoading, setIsUsenetLoading] = useState(true)
  const { isOpen, onOpen, onClose } = useDisclosure()
  const { isOpen: isInfoOpen, onOpen: onInfoOpen, onClose: onInfoClose } = useDisclosure()
  const { isOpen: isLinkOpen, onOpen: onLinkOpen, onClose: onLinkClose } = useDisclosure()
  const [selectedTorrentInfoHash, setSelectedTorrentInfoHash] = useState<string | null>(null)
  const [torrentToLink, setTorrentToLink] = useState<DownloadsTorrentRow | null>(null)

  const fetchTorrents = useCallback(async () => {
    try {
      const result = await graphqlClient
        .query<{
          Torrents: {
            Edges: Array<{
              Node: {
                Id: string
                InfoHash: string
                Name: string
                State: string
                Progress: number
                TotalBytes: number
                DownloadedBytes: number
                UploadedBytes: number
                SavePath: string
                AddedAt: string
              }
            }>
          }
        }>(DOWNLOADS_TORRENTS_QUERY, {
          Page: { Limit: 500, Offset: 0 },
        })
        .toPromise()
      if (result.data?.Torrents?.Edges) {
        const rows: DownloadsTorrentRow[] = result.data.Torrents.Edges.map(({ Node }) => ({
          id: Node.Id,
          infoHash: Node.InfoHash,
          name: Node.Name,
          state: (Node.State ?? '').toUpperCase(),
          progress: Node.Progress,
          size: Node.TotalBytes,
          downloaded: Node.DownloadedBytes,
          uploaded: Node.UploadedBytes,
          addedAt: Node.AddedAt,
        }))
        setTorrents(rows)
      }
      if (result.error) {
        const isAuthError = result.error.message?.toLowerCase().includes('authentication')
        if (!isAuthError) {
          addToast({
            title: 'Error',
            description: sanitizeError(result.error),
            color: 'danger',
          })
        }
      }
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e)
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

  // Fetch torrents and usenet downloads; subscribe to TorrentChanged to refetch list
  useEffect(() => {
    fetchTorrents()
    fetchUsenetDownloads()

    const sub = graphqlClient
      .subscription({ query: TorrentChangedDocument }, {})
      .subscribe({
        next: () => {
          fetchTorrents()
        },
      })

    return () => {
      sub.unsubscribe()
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
        .mutation<{ addTorrent?: AddTorrentResult; AddTorrent?: AddTorrentResult }>(ADD_TORRENT_MUTATION, {
          input: { magnet },
        })
        .toPromise()
      const data = result.data?.addTorrent ?? result.data?.AddTorrent
      const success = data?.success ?? data?.Success
      const torrent = data?.torrent ?? data?.Torrent
      const err = data?.error ?? data?.Error
      if (success && torrent) {
        const name = torrent.name ?? torrent.Name
        addToast({
          title: 'Torrent Added',
          description: `Started downloading: ${name}`,
          color: 'success',
        })
        fetchTorrents()
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(err ?? result.error?.message ?? 'Failed'),
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
        .mutation<{ addTorrent?: AddTorrentResult; AddTorrent?: AddTorrentResult }>(ADD_TORRENT_MUTATION, {
          input: { url },
        })
        .toPromise()
      const data = result.data?.addTorrent ?? result.data?.AddTorrent
      const success = data?.success ?? data?.Success
      const torrent = data?.torrent ?? data?.Torrent
      const err = data?.error ?? data?.Error
      if (success && torrent) {
        const name = torrent.name ?? torrent.Name
        addToast({
          title: 'Torrent Added',
          description: `Started downloading: ${name}`,
          color: 'success',
        })
        fetchTorrents()
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(err ?? result.error?.message ?? 'Failed'),
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

  // Single torrent actions (by infoHash – entity Torrents list)
  const handlePause = async (infoHash: string) => {
    const result = await graphqlClient
      .mutation<{ PauseTorrentByInfoHash?: { Success: boolean; Error?: string } }>(PAUSE_TORRENT_BY_INFO_HASH_MUTATION, {
        InfoHash: infoHash,
      })
      .toPromise()
    const data = result.data?.PauseTorrentByInfoHash
    if (data?.Success) {
      setTorrents((prev) =>
        prev.map((t) => (t.infoHash === infoHash ? { ...t, state: 'paused' } : t))
      )
    }
  }

  const handleResume = async (infoHash: string) => {
    const result = await graphqlClient
      .mutation<{ ResumeTorrentByInfoHash?: { Success: boolean; Error?: string } }>(RESUME_TORRENT_BY_INFO_HASH_MUTATION, {
        InfoHash: infoHash,
      })
      .toPromise()
    const data = result.data?.ResumeTorrentByInfoHash
    if (data?.Success) {
      setTorrents((prev) =>
        prev.map((t) => (t.infoHash === infoHash ? { ...t, state: 'downloading' } : t))
      )
    }
  }

  const handleRemove = async (infoHash: string) => {
    const result = await graphqlClient
      .mutation<{ RemoveTorrentByInfoHash?: { Success: boolean; Error?: string } }>(REMOVE_TORRENT_BY_INFO_HASH_MUTATION, {
        InfoHash: infoHash,
        DeleteFiles: false,
      })
      .toPromise()
    const data = result.data?.RemoveTorrentByInfoHash
    if (data?.Success) {
      setTorrents((prev) => prev.filter((t) => t.infoHash !== infoHash))
      addToast({
        title: 'Torrent Removed',
        description: 'The torrent has been removed.',
        color: 'success',
      })
    }
  }

  const handleInfo = (infoHash: string) => {
    setSelectedTorrentInfoHash(infoHash)
    onInfoOpen()
  }

  const handleOrganize = async (_infoHash: string) => {
    addToast({
      title: 'Organize',
      description: 'Organize by info hash is not yet available from this view.',
      color: 'default',
    })
  }

  // Process pending file matches (copy files to library)
  const handleProcess = async (torrent: DownloadsTorrentRow) => {
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
  const handleRematch = async (torrent: DownloadsTorrentRow) => {
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

  // Bulk actions (by infoHash)
  const handleBulkPause = async (infoHashes: string[]) => {
    let successCount = 0
    for (const infoHash of infoHashes) {
      const result = await graphqlClient
        .mutation<{ PauseTorrentByInfoHash?: { Success: boolean } }>(PAUSE_TORRENT_BY_INFO_HASH_MUTATION, {
          InfoHash: infoHash,
        })
        .toPromise()
      if (result.data?.PauseTorrentByInfoHash?.Success) {
        successCount++
        setTorrents((prev) =>
          prev.map((t) => (t.infoHash === infoHash ? { ...t, state: 'paused' } : t))
        )
      }
    }
    addToast({
      title: 'Paused Torrents',
      description: `Paused ${successCount} of ${infoHashes.length} torrent(s)`,
      color: 'success',
    })
  }

  const handleBulkResume = async (infoHashes: string[]) => {
    let successCount = 0
    for (const infoHash of infoHashes) {
      const result = await graphqlClient
        .mutation<{ ResumeTorrentByInfoHash?: { Success: boolean } }>(RESUME_TORRENT_BY_INFO_HASH_MUTATION, {
          InfoHash: infoHash,
        })
        .toPromise()
      if (result.data?.ResumeTorrentByInfoHash?.Success) {
        successCount++
        setTorrents((prev) =>
          prev.map((t) => (t.infoHash === infoHash ? { ...t, state: 'downloading' } : t))
        )
      }
    }
    addToast({
      title: 'Resumed Torrents',
      description: `Resumed ${successCount} of ${infoHashes.length} torrent(s)`,
      color: 'success',
    })
  }

  const handleBulkRemove = async (infoHashes: string[]) => {
    let successCount = 0
    for (const infoHash of infoHashes) {
      const result = await graphqlClient
        .mutation<{ RemoveTorrentByInfoHash?: { Success: boolean } }>(REMOVE_TORRENT_BY_INFO_HASH_MUTATION, {
          InfoHash: infoHash,
          DeleteFiles: false,
        })
        .toPromise()
      if (result.data?.RemoveTorrentByInfoHash?.Success) {
        successCount++
        setTorrents((prev) => prev.filter((t) => t.infoHash !== infoHash))
      }
    }
    addToast({
      title: 'Removed Torrents',
      description: `Removed ${successCount} of ${infoHashes.length} torrent(s)`,
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
        torrentInfoHash={selectedTorrentInfoHash}
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
