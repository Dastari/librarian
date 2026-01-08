import { createFileRoute, redirect } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import {
  Button,
  Card,
  CardBody,
  CardHeader,
  Input,
  Switch,
  Spinner,
  Divider,
} from '@heroui/react'
import {
  graphqlClient,
  TORRENT_SETTINGS_QUERY,
  UPDATE_TORRENT_SETTINGS_MUTATION,
  type TorrentSettings,
  type SettingsResult,
} from '../../lib/graphql'
import { useAuth } from '../../hooks/useAuth'
import { FolderBrowserInput } from '../../components/FolderBrowserInput'

export const Route = createFileRoute('/settings/')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/auth/login' })
    }
  },
  component: SettingsPage,
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

function SettingsPage() {
  const { session, loading: authLoading } = useAuth()
  const [_settings, setSettings] = useState<TorrentSettings | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)

  // Form state
  const [downloadDir, setDownloadDir] = useState('')
  const [sessionDir, setSessionDir] = useState('')
  const [enableDht, setEnableDht] = useState(true)
  const [listenPort, setListenPort] = useState(6881)
  const [maxConcurrent, setMaxConcurrent] = useState(5)
  const [uploadLimit, setUploadLimit] = useState(0)
  const [downloadLimit, setDownloadLimit] = useState(0)

  const fetchSettings = useCallback(async () => {
    try {
      const result = await graphqlClient.query(TORRENT_SETTINGS_QUERY, {}).toPromise()
      if (result.data?.torrentSettings) {
        const s = result.data.torrentSettings
        setSettings(s)
        setDownloadDir(s.downloadDir)
        setSessionDir(s.sessionDir)
        setEnableDht(s.enableDht)
        setListenPort(s.listenPort)
        setMaxConcurrent(s.maxConcurrent)
        setUploadLimit(s.uploadLimit)
        setDownloadLimit(s.downloadLimit)
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

  // Wait for auth to be ready before fetching settings
  useEffect(() => {
    // Don't fetch until auth has loaded
    if (authLoading) return
    
    // If not authenticated, show error
    if (!session) {
      setError('Authentication required')
      setIsLoading(false)
      return
    }

    fetchSettings()
  }, [authLoading, session, fetchSettings])

  const handleSave = async () => {
    setIsSaving(true)
    setError(null)
    setSuccess(null)

    try {
      const result = await graphqlClient
        .mutation<{ updateTorrentSettings: SettingsResult }>(UPDATE_TORRENT_SETTINGS_MUTATION, {
          input: {
            downloadDir,
            sessionDir,
            enableDht,
            listenPort,
            maxConcurrent,
            uploadLimit,
            downloadLimit,
          },
        })
        .toPromise()

      if (result.data?.updateTorrentSettings.success) {
        setSuccess('Settings saved successfully! Restart the server for changes to take effect.')
      } else {
        setError(sanitizeError(result.data?.updateTorrentSettings.error || 'Failed to save settings'))
      }
    } catch (e) {
      setError(sanitizeError(e))
    } finally {
      setIsSaving(false)
    }
  }

  const formatSpeed = (bytesPerSec: number) => {
    if (bytesPerSec === 0) return 'Unlimited'
    if (bytesPerSec >= 1024 * 1024) return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`
    if (bytesPerSec >= 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`
    return `${bytesPerSec} B/s`
  }

  if (isLoading) {
    return (
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="flex justify-center items-center py-20">
          <Spinner size="lg" />
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <h1 className="text-2xl font-bold mb-6">Settings</h1>

      {error && (
        <Card className="bg-danger-50 border-danger mb-6">
          <CardBody>
            <p className="text-danger">{error}</p>
          </CardBody>
        </Card>
      )}

      {success && (
        <Card className="bg-success-50 border-success mb-6">
          <CardBody>
            <p className="text-success">{success}</p>
          </CardBody>
        </Card>
      )}

      {/* Torrent Client Settings */}
      <Card className="mb-6">
        <CardHeader className="flex gap-3">
          <div className="flex flex-col">
            <p className="text-lg font-semibold">Torrent Client</p>
            <p className="text-small text-default-500">Configure the built-in torrent downloader</p>
          </div>
        </CardHeader>
        <Divider />
        <CardBody className="gap-6">
          {/* Download Directory */}
          <FolderBrowserInput
            label="Download Directory"
            value={downloadDir}
            onChange={setDownloadDir}
            placeholder="/data/downloads"
            description="Where downloaded files are saved. Make sure this path is writable."
            modalTitle="Select Download Directory"
          />

          {/* Session Directory */}
          <FolderBrowserInput
            label="Session Directory"
            value={sessionDir}
            onChange={setSessionDir}
            placeholder="/data/session"
            description="Where torrent session data (resume info, DHT cache) is stored."
            modalTitle="Select Session Directory"
          />

          <Divider />

          {/* DHT */}
          <div className="flex justify-between items-center">
            <div>
              <p className="font-medium">Enable DHT</p>
              <p className="text-xs text-default-400">
                Distributed Hash Table for finding peers without trackers
              </p>
            </div>
            <Switch isSelected={enableDht} onValueChange={setEnableDht} />
          </div>

          {/* Listen Port */}
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium">Listen Port</label>
            <Input
              type="number"
              value={listenPort.toString()}
              onChange={(e) => setListenPort(parseInt(e.target.value) || 0)}
              placeholder="6881"
              className="max-w-xs"
            />
            <p className="text-xs text-default-400">
              Port for incoming connections. Set to 0 for random port.
            </p>
          </div>

          {/* Max Concurrent */}
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium">Max Concurrent Downloads</label>
            <Input
              type="number"
              value={maxConcurrent.toString()}
              onChange={(e) => setMaxConcurrent(parseInt(e.target.value) || 1)}
              placeholder="5"
              className="max-w-xs"
              min={1}
              max={20}
            />
            <p className="text-xs text-default-400">
              Maximum number of torrents downloading simultaneously
            </p>
          </div>

          <Divider />

          {/* Speed Limits */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="flex flex-col gap-2">
              <label className="text-sm font-medium">
                Download Limit: {formatSpeed(downloadLimit)}
              </label>
              <Input
                type="number"
                value={downloadLimit.toString()}
                onChange={(e) => setDownloadLimit(parseInt(e.target.value) || 0)}
                placeholder="0"
                endContent={<span className="text-default-400 text-sm">B/s</span>}
              />
              <p className="text-xs text-default-400">0 = unlimited</p>
            </div>
            <div className="flex flex-col gap-2">
              <label className="text-sm font-medium">
                Upload Limit: {formatSpeed(uploadLimit)}
              </label>
              <Input
                type="number"
                value={uploadLimit.toString()}
                onChange={(e) => setUploadLimit(parseInt(e.target.value) || 0)}
                placeholder="0"
                endContent={<span className="text-default-400 text-sm">B/s</span>}
              />
              <p className="text-xs text-default-400">0 = unlimited</p>
            </div>
          </div>
        </CardBody>
      </Card>

      {/* Save Button */}
      <div className="flex justify-end gap-3">
        <Button variant="flat" onPress={fetchSettings} isDisabled={isSaving}>
          Reset
        </Button>
        <Button color="primary" onPress={handleSave} isLoading={isSaving}>
          Save Settings
        </Button>
      </div>

    </div>
  )
}
