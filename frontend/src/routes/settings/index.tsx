import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody, CardHeader } from '@heroui/card'
import { Input } from '@heroui/input'
import { Switch } from '@heroui/switch'
import { Divider } from '@heroui/divider'
import { addToast } from '@heroui/toast'
import {
  graphqlClient,
  TORRENT_SETTINGS_QUERY,
  UPDATE_TORRENT_SETTINGS_MUTATION,
  type TorrentSettings,
  type SettingsResult,
} from '../../lib/graphql'
import { FolderBrowserInput } from '../../components/FolderBrowserInput'
import { sanitizeError } from '../../lib/format'

export const Route = createFileRoute('/settings/')({
  component: SettingsPage,
})

function SettingsPage() {
  const [_settings, setSettings] = useState<TorrentSettings | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isSaving, setIsSaving] = useState(false)

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
      const result = await graphqlClient.query<{ torrentSettings: TorrentSettings }>(TORRENT_SETTINGS_QUERY, {}).toPromise()
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

  // Fetch settings on mount
  useEffect(() => {
    fetchSettings()
  }, [fetchSettings])

  const handleSave = async () => {
    setIsSaving(true)

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
        addToast({
          title: 'Settings Saved',
          description: 'Restart the server for changes to take effect.',
          color: 'success',
        })
      } else {
        addToast({
          title: 'Error',
          description: sanitizeError(result.data?.updateTorrentSettings.error || 'Failed to save settings'),
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
      setIsSaving(false)
    }
  }

  const formatSpeed = (bytesPerSec: number) => {
    if (bytesPerSec === 0) return 'Unlimited'
    if (bytesPerSec >= 1024 * 1024) return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`
    if (bytesPerSec >= 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`
    return `${bytesPerSec} B/s`
  }

  return (
    <>
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
            isDisabled={isLoading}
          />

          {/* Session Directory */}
          <FolderBrowserInput
            label="Session Directory"
            value={sessionDir}
            onChange={setSessionDir}
            placeholder="/data/session"
            description="Where torrent session data (resume info, DHT cache) is stored."
            modalTitle="Select Session Directory"
            isDisabled={isLoading}
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
            <Switch isSelected={enableDht} onValueChange={setEnableDht} isDisabled={isLoading} />
          </div>

          {/* Listen Port */}
          <Input
            type="number"
            label="Listen Port"
            description="Port for incoming connections. Set to 0 for random port."
            value={listenPort.toString()}
            onChange={(e) => setListenPort(parseInt(e.target.value) || 0)}
            placeholder="6881"
            className="max-w-xs"
            isDisabled={isLoading}
          />

          {/* Max Concurrent */}
          <Input
            type="number"
            label="Max Concurrent Downloads"
            description="Maximum number of torrents downloading simultaneously"
            value={maxConcurrent.toString()}
            onChange={(e) => setMaxConcurrent(parseInt(e.target.value) || 1)}
            placeholder="5"
            className="max-w-xs"
            min={1}
            max={20}
            isDisabled={isLoading}
          />

          <Divider />

          {/* Speed Limits */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <Input
              type="number"
              label={`Download Limit: ${formatSpeed(downloadLimit)}`}
              description="0 = unlimited"
              value={downloadLimit.toString()}
              onChange={(e) => setDownloadLimit(parseInt(e.target.value) || 0)}
              placeholder="0"
              endContent={<span className="text-default-400 text-sm">B/s</span>}
              isDisabled={isLoading}
            />
            <Input
              type="number"
              label={`Upload Limit: ${formatSpeed(uploadLimit)}`}
              description="0 = unlimited"
              value={uploadLimit.toString()}
              onChange={(e) => setUploadLimit(parseInt(e.target.value) || 0)}
              placeholder="0"
              endContent={<span className="text-default-400 text-sm">B/s</span>}
              isDisabled={isLoading}
            />
          </div>
        </CardBody>
      </Card>

      {/* Save Button */}
      <div className="flex justify-end gap-3">
        <Button variant="flat" onPress={fetchSettings} isDisabled={isSaving || isLoading}>
          Reset
        </Button>
        <Button color="primary" onPress={handleSave} isLoading={isSaving} isDisabled={isLoading}>
          Save Settings
        </Button>
      </div>
    </>
  )
}
