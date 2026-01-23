import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useMemo } from 'react'
import { Input } from '@heroui/input'
import { Switch } from '@heroui/switch'
import { Accordion, AccordionItem } from '@heroui/accordion'
import { Button } from '@heroui/button'
import { addToast } from '@heroui/toast'
import {
  graphqlClient,
  TORRENT_SETTINGS_QUERY,
  UPDATE_TORRENT_SETTINGS_MUTATION,
  UPnP_STATUS_QUERY,
  TEST_PORT_ACCESSIBILITY_QUERY,
  ATTEMPT_UPNP_PORT_FORWARDING_MUTATION,
  type TorrentSettings,
  type SettingsResult,
  type UpnpResult,
  type PortTestResult,
} from '../../lib/graphql'
import { FolderBrowserInput } from '../../components/FolderBrowserInput'
import { SettingsHeader } from '../../components/shared'
import { sanitizeError } from '../../lib/format'
import { IconFolder, IconNetwork, IconGauge, IconTestPipe, IconAlertTriangle, IconCheck, IconX } from '@tabler/icons-react'

export const Route = createFileRoute('/settings/torrent')({
  component: TorrentSettingsPage,
})

function TorrentSettingsPage() {
  const [originalSettings, setOriginalSettings] = useState<TorrentSettings | null>(null)
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

  // UPnP and port testing state
  const [upnpStatus, setUpnpStatus] = useState<UpnpResult | null>(null)
  const [portTestResult, setPortTestResult] = useState<PortTestResult | null>(null)
  const [isTestingPort, setIsTestingPort] = useState(false)
  const [isAttemptingUpnp, setIsAttemptingUpnp] = useState(false)

  // Track changes
  const hasChanges = useMemo(() => {
    if (!originalSettings) return false
    return (
      downloadDir !== originalSettings.downloadDir ||
      sessionDir !== originalSettings.sessionDir ||
      enableDht !== originalSettings.enableDht ||
      listenPort !== originalSettings.listenPort ||
      maxConcurrent !== originalSettings.maxConcurrent ||
      uploadLimit !== originalSettings.uploadLimit ||
      downloadLimit !== originalSettings.downloadLimit
    )
  }, [originalSettings, downloadDir, sessionDir, enableDht, listenPort, maxConcurrent, uploadLimit, downloadLimit])

  const fetchSettings = useCallback(async () => {
    try {
      const result = await graphqlClient.query<{ torrentSettings: TorrentSettings }>(TORRENT_SETTINGS_QUERY, {}).toPromise()
      if (result.data?.torrentSettings) {
        const s = result.data.torrentSettings
        setOriginalSettings(s)
        setDownloadDir(s.downloadDir)
        setSessionDir(s.sessionDir)
        setEnableDht(s.enableDht)
        setListenPort(s.listenPort)
        setMaxConcurrent(s.maxConcurrent)
        setUploadLimit(s.uploadLimit)
        setDownloadLimit(s.downloadLimit)
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

  // Fetch settings on mount
  useEffect(() => {
    fetchSettings()
  }, [fetchSettings])

  const fetchUpnpStatus = useCallback(async () => {
    try {
      const result = await graphqlClient.query<{ upnpStatus: UpnpResult | null }>(UPnP_STATUS_QUERY, {}).toPromise()
      if (result.data?.upnpStatus) {
        setUpnpStatus(result.data.upnpStatus)
      }
    } catch (e) {
      // Silently ignore errors for UPnP status
      console.warn('Failed to fetch UPnP status:', e)
    }
  }, [])

  const testPortAccessibility = useCallback(async () => {
    setIsTestingPort(true)
    try {
      const result = await graphqlClient.query<{ testPortAccessibility: PortTestResult }>(TEST_PORT_ACCESSIBILITY_QUERY, {
        port: listenPort
      }).toPromise()
      if (result.data?.testPortAccessibility) {
        setPortTestResult(result.data.testPortAccessibility)
        addToast({
          title: result.data.testPortAccessibility.portOpen ? 'Port Test Successful' : 'Port Test Failed',
          description: result.data.testPortAccessibility.portOpen
            ? `Port ${listenPort} is accessible from the internet`
            : `Port ${listenPort} is not accessible from the internet`,
          color: result.data.testPortAccessibility.portOpen ? 'success' : 'warning',
        })
      }
    } catch (e) {
      addToast({
        title: 'Port Test Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    } finally {
      setIsTestingPort(false)
    }
  }, [listenPort])

  const attemptUpnpForwarding = useCallback(async () => {
    setIsAttemptingUpnp(true)
    try {
      const result = await graphqlClient
        .mutation<{ attemptUpnpPortForwarding: UpnpResult }>(ATTEMPT_UPNP_PORT_FORWARDING_MUTATION, {})
        .toPromise()

      if (result.data?.attemptUpnpPortForwarding) {
        const upnpResult = result.data.attemptUpnpPortForwarding
        setUpnpStatus(upnpResult)

        addToast({
          title: upnpResult.success ? 'UPnP Port Forwarding Successful' : 'UPnP Port Forwarding Failed',
          description: upnpResult.success
            ? `Successfully forwarded port ${listenPort} via UPnP`
            : `Failed to forward port ${listenPort} via UPnP: ${upnpResult.error || 'Unknown error'}`,
          color: upnpResult.success ? 'success' : 'warning',
        })
      }
    } catch (e) {
      addToast({
        title: 'UPnP Error',
        description: sanitizeError(e),
        color: 'danger',
      })
    } finally {
      setIsAttemptingUpnp(false)
    }
  }, [listenPort])

  // Fetch UPnP status on mount
  useEffect(() => {
    fetchUpnpStatus()
  }, [fetchUpnpStatus])

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

  const handleReset = useCallback(() => {
    if (originalSettings) {
      setDownloadDir(originalSettings.downloadDir)
      setSessionDir(originalSettings.sessionDir)
      setEnableDht(originalSettings.enableDht)
      setListenPort(originalSettings.listenPort)
      setMaxConcurrent(originalSettings.maxConcurrent)
      setUploadLimit(originalSettings.uploadLimit)
      setDownloadLimit(originalSettings.downloadLimit)
    }
  }, [originalSettings])

  const formatSpeed = (bytesPerSec: number) => {
    if (bytesPerSec === 0) return 'Unlimited'
    if (bytesPerSec >= 1024 * 1024) return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`
    if (bytesPerSec >= 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`
    return `${bytesPerSec} B/s`
  }

  return (
    <div className="grow overflow-y-auto overflow-x-hidden pb-8" style={{ scrollbarGutter: 'stable' }}>
      <SettingsHeader
        title="Torrent Client"
        subtitle="Configure the built-in torrent downloader"
        onSave={handleSave}
        onReset={handleReset}
        isSaveDisabled={!hasChanges || isLoading}
        isResetDisabled={!hasChanges}
        isSaving={isSaving}
        hasChanges={hasChanges}
      />

      <Accordion
        selectionMode="multiple"
        // defaultExpandedKeys={['directories', 'network', 'limits']}
        variant="splitted"
      >
        {/* Directories Section */}
        <AccordionItem
          key="directories"
          aria-label="Directories"
          title={
            <div className="flex items-center gap-2">
              <IconFolder size={18} className="text-amber-400" />
              <span className="font-semibold">Directories</span>
            </div>
          }
          subtitle="Download and session storage locations"
        >
          <div className="space-y-4 pb-2">
            <FolderBrowserInput
              label="Download Directory"
              value={downloadDir}
              onChange={setDownloadDir}
              placeholder="/data/downloads"
              description="Where downloaded files are saved. Make sure this path is writable."
              modalTitle="Select Download Directory"
              isDisabled={isLoading}
            />

            <FolderBrowserInput
              label="Session Directory"
              value={sessionDir}
              onChange={setSessionDir}
              placeholder="/data/session"
              description="Where torrent session data (resume info, DHT cache) is stored."
              modalTitle="Select Session Directory"
              isDisabled={isLoading}
            />
          </div>
        </AccordionItem>

        {/* Network Section */}
        <AccordionItem
          key="network"
          aria-label="Network"
          title={
            <div className="flex items-center gap-2">
              <IconNetwork size={18} className="text-blue-400" />
              <span className="font-semibold">Network</span>
            </div>
          }
          subtitle="DHT, ports, and connection settings"
        >
          <div className="space-y-4 pb-2">
            <div className="flex justify-between items-center">
              <div>
                <p className="font-medium">Enable DHT</p>
                <p className="text-xs text-default-400">
                  Distributed Hash Table for finding peers without trackers
                </p>
              </div>
              <Switch isSelected={enableDht} onValueChange={setEnableDht} isDisabled={isLoading} />
            </div>

            <Input
              type="number"
              label="Listen Port"
              labelPlacement="inside"
              variant="flat"
              description="Port for incoming connections. Set to 0 for random port."
              value={listenPort.toString()}
              onChange={(e) => setListenPort(parseInt(e.target.value) || 0)}
              placeholder="6881"
              className="max-w-xs"
              isDisabled={isLoading}
              classNames={{
                label: 'text-sm font-medium text-primary!',
              }}
            />

            <Input
              type="number"
              label="Max Concurrent Downloads"
              labelPlacement="inside"
              variant="flat"
              description="Maximum number of torrents downloading simultaneously"
              value={maxConcurrent.toString()}
              onChange={(e) => setMaxConcurrent(parseInt(e.target.value) || 1)}
              placeholder="5"
              className="max-w-xs"
              min={1}
              max={20}
              isDisabled={isLoading}
              classNames={{
                label: 'text-sm font-medium text-primary!',
              }}
            />

            {/* UPnP Port Forwarding Status */}
            <div className="space-y-3 p-4 bg-content2 rounded-lg">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium text-foreground">UPnP Port Forwarding</p>
                  <p className="text-xs text-default-500">
                    Automatic port forwarding via UPnP for better connectivity
                  </p>
                </div>
                <div className="flex gap-2">
                  {upnpStatus && (
                    <div className="flex items-center gap-1 text-xs">
                      {upnpStatus.tcpForwarded || upnpStatus.udpForwarded ? (
                        <>
                          <IconCheck size={14} className="text-green-400" />
                          <span className="text-success">Active</span>
                        </>
                      ) : (
                        <>
                          <IconX size={14} className="text-red-400" />
                          <span className="text-danger">Failed</span>
                        </>
                      )}
                    </div>
                  )}
                  <Button
                    size="sm"
                    variant="flat"
                    color="primary"
                    isLoading={isAttemptingUpnp}
                    onPress={attemptUpnpForwarding}
                    startContent={<IconTestPipe size={14} />}
                  >
                    Test UPnP
                  </Button>
                </div>
              </div>

              {/* Alert Banner when UPnP fails */}
              {upnpStatus && !upnpStatus.success && !isAttemptingUpnp && (
                <div className="flex items-start gap-3 p-3 bg-warning-50 border border-warning-200 rounded-md">
                  <IconAlertTriangle size={16} className="text-warning mt-0.5 flex-shrink-0" />
                  <div className="flex-1">
                    <p className="text-sm font-medium text-warning-800">
                      Port forwarding required
                    </p>
                    <p className="text-xs text-warning-700 mt-1">
                      UPnP port forwarding failed. To improve torrent performance, create a port forwarding rule
                      in your router for port {listenPort} (TCP and UDP) to your backend's local IP address.
                    </p>
                  </div>
                </div>
              )}

              {/* Port Accessibility Test */}
              <div className="flex items-center justify-between pt-2 border-t border-default-200">
                <div>
                  <p className="text-sm font-medium text-foreground">Port Accessibility</p>
                  <p className="text-xs text-default-500">
                    Test if port {listenPort} is accessible from the internet
                  </p>
                </div>
                <Button
                  size="sm"
                  variant="flat"
                  color="secondary"
                  isLoading={isTestingPort}
                  onPress={testPortAccessibility}
                  startContent={<IconTestPipe size={14} />}
                >
                  Test Port
                </Button>
              </div>

              {portTestResult && (
                <div className={`flex items-center gap-2 p-2 rounded text-xs ${
                  portTestResult.portOpen
                    ? 'bg-success-50 text-success-700'
                    : 'bg-danger-50 text-danger-700'
                }`}>
                  {portTestResult.portOpen ? (
                    <IconCheck size={14} />
                  ) : (
                    <IconX size={14} />
                  )}
                  <span>
                    Port {listenPort} is {portTestResult.portOpen ? 'accessible' : 'not accessible'} from the internet
                    {portTestResult.externalIp && (
                      <span className="ml-1 text-default-500">
                        (External IP: {portTestResult.externalIp})
                      </span>
                    )}
                  </span>
                </div>
              )}
            </div>
          </div>
        </AccordionItem>

        {/* Speed Limits Section */}
        <AccordionItem
          key="limits"
          aria-label="Speed Limits"
          title={
            <div className="flex items-center gap-2">
              <IconGauge size={18} className="text-green-400" />
              <span className="font-semibold">Speed Limits</span>
            </div>
          }
          subtitle="Bandwidth throttling for uploads and downloads"
        >
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pb-2">
            <Input
              type="number"
              label={`Download Limit: ${formatSpeed(downloadLimit)}`}
              labelPlacement="inside"
              variant="flat"
              description="0 = unlimited"
              value={downloadLimit.toString()}
              onChange={(e) => setDownloadLimit(parseInt(e.target.value) || 0)}
              placeholder="0"
              endContent={<span className="text-default-400 text-sm">B/s</span>}
              isDisabled={isLoading}
              classNames={{
                label: 'text-sm font-medium text-primary!',
              }}
            />
            <Input
              type="number"
              label={`Upload Limit: ${formatSpeed(uploadLimit)}`}
              labelPlacement="inside"
              variant="flat"
              description="0 = unlimited"
              value={uploadLimit.toString()}
              onChange={(e) => setUploadLimit(parseInt(e.target.value) || 0)}
              placeholder="0"
              endContent={<span className="text-default-400 text-sm">B/s</span>}
              isDisabled={isLoading}
              classNames={{
                label: 'text-sm font-medium text-primary!',
              }}
            />
          </div>
        </AccordionItem>
      </Accordion>
    </div>
  )
}
