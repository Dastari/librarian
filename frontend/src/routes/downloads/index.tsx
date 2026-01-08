import { createFileRoute, redirect } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import {
  Button,
  Card,
  CardBody,
  Input,
  Progress,
  Spinner,
  Tabs,
  Tab,
  Chip,
  Tooltip,
} from '@heroui/react'
import {
  graphqlClient,
  TORRENTS_QUERY,
  ADD_TORRENT_MUTATION,
  PAUSE_TORRENT_MUTATION,
  RESUME_TORRENT_MUTATION,
  REMOVE_TORRENT_MUTATION,
  TORRENT_PROGRESS_SUBSCRIPTION,
  TORRENT_ADDED_SUBSCRIPTION,
  TORRENT_REMOVED_SUBSCRIPTION,
  type Torrent,
  type TorrentProgress,
  type AddTorrentResult,
  type TorrentActionResult,
} from '../../lib/graphql'
import { useAuth } from '../../hooks/useAuth'

export const Route = createFileRoute('/downloads/')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/auth/login' })
    }
  },
  component: DownloadsPage,
})

// Helper to sanitize error messages (avoid displaying raw HTML)
function sanitizeError(error: unknown): string {
  if (!error) return 'Unknown error'
  const message = typeof error === 'string' ? error : (error as Error).message || String(error)
  // If the error contains HTML, show a generic message
  if (message.includes('<!DOCTYPE') || message.includes('<html')) {
    return 'Failed to connect to server. Please check that the backend is running.'
  }
  // Truncate very long messages
  if (message.length > 200) {
    return message.substring(0, 200) + '...'
  }
  return message
}

function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let unitIndex = 0
  let size = bytes
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }
  return `${size.toFixed(1)} ${units[unitIndex]}`
}

function formatSpeed(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`
}

type InputMode = 'magnet' | 'url' | 'file'

function DownloadsPage() {
  const { session, loading: authLoading } = useAuth()
  const [torrents, setTorrents] = useState<Torrent[]>([])
  const [magnetUrl, setMagnetUrl] = useState('')
  const [torrentUrl, setTorrentUrl] = useState('')
  const [inputMode, setInputMode] = useState<InputMode>('magnet')
  const [isAdding, setIsAdding] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isDragging, setIsDragging] = useState(false)

  const fetchTorrents = useCallback(async () => {
    try {
      const result = await graphqlClient.query(TORRENTS_QUERY, {}).toPromise()
      if (result.data?.torrents) {
        setTorrents(result.data.torrents)
        setError(null)
      }
      if (result.error) {
        setError(sanitizeError(result.error))
      }
    } catch (e) {
      setError(sanitizeError(e))
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Wait for auth to be ready before fetching torrents
  useEffect(() => {
    // Don't fetch until auth has loaded
    if (authLoading) return
    
    // If not authenticated, show error
    if (!session) {
      setError('Authentication required')
      setIsLoading(false)
      return
    }

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
  }, [authLoading, session, fetchTorrents])

  const handleAddTorrent = async (e: React.FormEvent) => {
    e.preventDefault()
    const input = inputMode === 'magnet' ? magnetUrl.trim() : torrentUrl.trim()
    if (!input) return

    setIsAdding(true)
    setError(null)
    try {
      const result = await graphqlClient
        .mutation<{ addTorrent: AddTorrentResult }>(ADD_TORRENT_MUTATION, {
          input: inputMode === 'magnet' ? { magnet: input } : { url: input },
        })
        .toPromise()
      if (result.data?.addTorrent.success && result.data.addTorrent.torrent) {
        setTorrents((prev) => [result.data!.addTorrent.torrent!, ...prev])
        setMagnetUrl('')
        setTorrentUrl('')
      } else {
        setError(sanitizeError(result.data?.addTorrent.error || result.error?.message || 'Failed'))
      }
    } catch {
      setError('Failed to add torrent')
    } finally {
      setIsAdding(false)
    }
  }

  const handleFileUpload = async (file: File) => {
    setIsAdding(true)
    setError(null)

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
      } else {
        setError(data.error || 'Failed to upload torrent file')
      }
    } catch (e) {
      setError('Failed to upload torrent file')
      console.error(e)
    } finally {
      setIsAdding(false)
    }
  }

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)

    const files = e.dataTransfer.files
    if (files.length > 0) {
      const file = files[0]
      if (file.name.endsWith('.torrent')) {
        handleFileUpload(file)
      } else {
        setError('Please drop a .torrent file')
      }
    }
  }

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files
    if (files && files.length > 0) {
      handleFileUpload(files[0])
      e.target.value = ''
    }
  }

  const handlePause = async (id: number) => {
    await graphqlClient
      .mutation<{ pauseTorrent: TorrentActionResult }>(PAUSE_TORRENT_MUTATION, { id })
      .toPromise()
  }

  const handleResume = async (id: number) => {
    await graphqlClient
      .mutation<{ resumeTorrent: TorrentActionResult }>(RESUME_TORRENT_MUTATION, { id })
      .toPromise()
  }

  const handleRemove = async (id: number) => {
    if (!confirm('Remove this torrent?')) return
    const result = await graphqlClient
      .mutation<{ removeTorrent: TorrentActionResult }>(REMOVE_TORRENT_MUTATION, {
        id,
        deleteFiles: false,
      })
      .toPromise()
    if (result.data?.removeTorrent.success) {
      setTorrents((prev) => prev.filter((t) => t.id !== id))
    }
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <h1 className="text-2xl font-bold mb-6">Downloads</h1>

      {/* Error message */}
      {error && (
        <Card className="bg-danger-50 border-danger mb-6">
          <CardBody className="flex-row justify-between items-center">
            <p className="text-danger">{error}</p>
            <Button size="sm" variant="light" color="danger" onPress={() => setError(null)}>
              Dismiss
            </Button>
          </CardBody>
        </Card>
      )}

      {/* Add torrent tabs */}
      <Card className="mb-8">
        <CardBody>
          <Tabs
            selectedKey={inputMode}
            onSelectionChange={(key) => setInputMode(key as InputMode)}
            aria-label="Add torrent options"
          >
            <Tab key="magnet" title="🧲 Magnet Link">
              <form onSubmit={handleAddTorrent} className="pt-4">
                <div className="flex gap-4">
                  <Input
                    value={magnetUrl}
                    onChange={(e) => setMagnetUrl(e.target.value)}
                    placeholder="Paste magnet link (magnet:?xt=urn:btih:...)..."
                    className="flex-1"
                    size="lg"
                  />
                  <Button
                    type="submit"
                    color="primary"
                    size="lg"
                    isLoading={isAdding}
                    isDisabled={!magnetUrl.trim()}
                  >
                    Add
                  </Button>
                </div>
              </form>
            </Tab>
            <Tab key="url" title="🔗 Torrent URL">
              <form onSubmit={handleAddTorrent} className="pt-4">
                <div className="flex gap-4">
                  <Input
                    value={torrentUrl}
                    onChange={(e) => setTorrentUrl(e.target.value)}
                    placeholder="Enter URL to .torrent file (https://...)..."
                    className="flex-1"
                    size="lg"
                  />
                  <Button
                    type="submit"
                    color="primary"
                    size="lg"
                    isLoading={isAdding}
                    isDisabled={!torrentUrl.trim()}
                  >
                    Add
                  </Button>
                </div>
              </form>
            </Tab>
            <Tab key="file" title="📁 Upload File">
              <div
                onDragOver={(e) => {
                  e.preventDefault()
                  setIsDragging(true)
                }}
                onDragLeave={() => setIsDragging(false)}
                onDrop={handleDrop}
                className={`mt-4 border-2 border-dashed rounded-xl p-8 text-center transition-colors ${
                  isDragging ? 'border-primary bg-primary/10' : 'border-default-300'
                }`}
              >
                <input
                  type="file"
                  accept=".torrent"
                  onChange={handleFileSelect}
                  className="hidden"
                  id="torrent-file-input"
                />
                <label htmlFor="torrent-file-input" className="cursor-pointer">
                  <div className="text-4xl mb-4">📁</div>
                  <p className="text-default-600 mb-2">
                    {isDragging ? 'Drop your .torrent file here!' : 'Drag & drop a .torrent file'}
                  </p>
                  <p className="text-default-400 text-sm mb-4">or</p>
                  <Button color="primary" as="span" isLoading={isAdding}>
                    Browse Files
                  </Button>
                </label>
              </div>
            </Tab>
          </Tabs>
        </CardBody>
      </Card>

      {/* Loading */}
      {isLoading && (
        <div className="flex justify-center py-12">
          <Spinner size="lg" />
        </div>
      )}

      {/* Torrents list */}
      {!isLoading && (
        <div className="space-y-4">
          {torrents.map((t) => (
            <TorrentCard
              key={t.id}
              torrent={t}
              onPause={() => handlePause(t.id)}
              onResume={() => handleResume(t.id)}
              onRemove={() => handleRemove(t.id)}
            />
          ))}

          {torrents.length === 0 && (
            <Card>
              <CardBody className="text-center py-12">
                <p className="text-lg text-default-600 mb-2">No active downloads</p>
                <p className="text-sm text-default-400">
                  Add a torrent using the options above to start downloading.
                </p>
              </CardBody>
            </Card>
          )}
        </div>
      )}
    </div>
  )
}

function TorrentCard({
  torrent,
  onPause,
  onResume,
  onRemove,
}: {
  torrent: Torrent
  onPause: () => void
  onResume: () => void
  onRemove: () => void
}) {
  const isPaused = torrent.state === 'PAUSED'
  const isSeeding = torrent.state === 'SEEDING'
  const isError = torrent.state === 'ERROR'
  const isDownloading = torrent.state === 'DOWNLOADING'

  const progressColor = isSeeding
    ? 'success'
    : isError
      ? 'danger'
      : isPaused
        ? 'warning'
        : 'primary'

  const stateLabels: Record<string, { label: string; color: 'default' | 'primary' | 'success' | 'warning' | 'danger' }> = {
    QUEUED: { label: 'Queued', color: 'default' },
    CHECKING: { label: 'Checking', color: 'primary' },
    DOWNLOADING: { label: 'Downloading', color: 'primary' },
    SEEDING: { label: 'Seeding', color: 'success' },
    PAUSED: { label: 'Paused', color: 'warning' },
    ERROR: { label: 'Error', color: 'danger' },
  }

  const stateInfo = stateLabels[torrent.state] || stateLabels.QUEUED

  return (
    <Card>
      <CardBody>
        <div className="flex items-start justify-between mb-3">
          <div className="flex-1 min-w-0 mr-4">
            <h3 className="font-semibold truncate" title={torrent.name}>
              {torrent.name}
            </h3>
            <div className="flex items-center gap-2 mt-1">
              <span className="text-sm text-default-500">
                {torrent.sizeFormatted || formatBytes(torrent.size)}
              </span>
              <Chip size="sm" color={stateInfo.color} variant="flat">
                {stateInfo.label}
              </Chip>
            </div>
          </div>
          <div className="flex gap-1">
            {isPaused ? (
              <Tooltip content="Resume">
                <Button isIconOnly size="sm" variant="light" color="success" onPress={onResume}>
                  ▶️
                </Button>
              </Tooltip>
            ) : isDownloading ? (
              <Tooltip content="Pause">
                <Button isIconOnly size="sm" variant="light" color="warning" onPress={onPause}>
                  ⏸️
                </Button>
              </Tooltip>
            ) : null}
            <Tooltip content="Remove">
              <Button isIconOnly size="sm" variant="light" color="danger" onPress={onRemove}>
                🗑️
              </Button>
            </Tooltip>
          </div>
        </div>

        <Progress
          value={torrent.progress * 100}
          color={progressColor}
          size="md"
          className="mb-2"
          aria-label="Download progress"
        />

        <div className="flex justify-between text-sm text-default-500">
          <span>{(torrent.progress * 100).toFixed(1)}%</span>
          <span>
            {isDownloading && (
              <>
                ⬇️ {formatSpeed(torrent.downloadSpeed)} • ⬆️ {formatSpeed(torrent.uploadSpeed)}
                {torrent.peers > 0 && ` • ${torrent.peers} peers`}
              </>
            )}
            {isSeeding && (
              <>
                ⬆️ {formatSpeed(torrent.uploadSpeed)}
                {torrent.peers > 0 && ` • ${torrent.peers} peers`}
              </>
            )}
          </span>
        </div>
      </CardBody>
    </Card>
  )
}
