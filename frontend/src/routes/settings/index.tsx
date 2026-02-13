import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect } from 'react'
import { Card, CardBody, CardHeader } from '@heroui/card'
import { Slider } from '@heroui/slider'
import { Button } from '@heroui/button'
import { Spinner } from '@heroui/spinner'
import { Divider } from '@heroui/divider'
import { addToast } from '@heroui/toast'
import { IconPlayerPlay, IconDeviceFloppy, IconServer2, IconRefresh } from '@tabler/icons-react'
import {
  PlaybackSyncIntervalDocument,
  UpdateAppSettingDocument,
} from '../../lib/graphql'
import { useQuery, useMutation } from '../../lib/graphql/client'
import { authFetch } from '../../lib/api/authFetch'

export const Route = createFileRoute('/settings/')({
  component: GeneralSettingsPage,
})

const PLAYBACK_SYNC_KEY = 'playback_sync_interval'
type HealthResponse = {
  status: string
  version: string
  platform: string
  network_paths: Array<{
    path: string
    reachable: boolean
    message?: string | null
  }>
}

function GeneralSettingsPage() {
  const [savedSyncInterval, setSavedSyncInterval] = useState<number | null>(null)
  const [syncInterval, setSyncInterval] = useState(15)
  const [health, setHealth] = useState<HealthResponse | null>(null)
  const [isHealthLoading, setIsHealthLoading] = useState(true)
  const [isRefreshingHealth, setIsRefreshingHealth] = useState(false)

  const { data, previousData, loading } = useQuery(PlaybackSyncIntervalDocument, {
    variables: { Key: PLAYBACK_SYNC_KEY },
    fetchPolicy: 'cache-and-network',
  })

  const [updateAppSetting, { loading: saving }] = useMutation(UpdateAppSettingDocument)

  const node = data?.AppSettings?.Edges?.[0]?.Node ?? previousData?.AppSettings?.Edges?.[0]?.Node
  const settingId = node?.Id ?? null

  useEffect(() => {
    if (!node) return
    const val = Number(node.Value)
    if (Number.isFinite(val)) {
      setSyncInterval(val)
      setSavedSyncInterval(val)
    }
  }, [node?.Id, node?.Value])

  const loadHealth = async (isRefresh = false) => {
    try {
      if (isRefresh) {
        setIsRefreshingHealth(true)
      } else {
        setIsHealthLoading(true)
      }

      const res = await authFetch('/api/healthz')
      if (!res.ok) {
        throw new Error(`Health check failed (${res.status})`)
      }
      const json = (await res.json()) as HealthResponse
      setHealth(json)
    } catch (err) {
      console.error('Failed to load health status:', err)
    } finally {
      setIsHealthLoading(false)
      setIsRefreshingHealth(false)
    }
  }

  useEffect(() => {
    void loadHealth(false)
  }, [])

  const handleSave = async () => {
    if (settingId == null) return
    try {
      const result = await updateAppSetting({
        variables: {
          Id: settingId,
          Input: { Value: String(syncInterval) },
        },
      })

      const payload = result.data?.UpdateAppSetting
      if (payload?.Success) {
        setSavedSyncInterval(syncInterval)
        addToast({
          title: 'Settings Saved',
          description: 'Playback settings have been updated',
          color: 'success',
        })
      } else {
        addToast({
          title: 'Error',
          description: payload?.Error ?? 'Failed to save playback settings',
          color: 'danger',
        })
      }
    } catch (err) {
      console.error('Failed to save settings:', err)
      addToast({
        title: 'Error',
        description: 'Failed to save playback settings',
        color: 'danger',
      })
    }
  }

  const hasChanges = savedSyncInterval !== null && syncInterval !== savedSyncInterval

  if (loading) {
    return (
      <div className="flex flex-col gap-6">
        <div>
          <h2 className="text-xl font-semibold">General</h2>
          <p className="text-default-500 text-sm">
            Application-wide settings and preferences
          </p>
        </div>
        <Card>
          <CardBody className="py-16 flex items-center justify-center">
            <Spinner size="lg" />
          </CardBody>
        </Card>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-6">
      {/* Page Header */}
      <div>
        <h2 className="text-xl font-semibold">General</h2>
        <p className="text-default-500 text-sm">
          Application-wide settings and preferences
        </p>
      </div>

      {/* Playback Settings */}
      <Card>
        <CardHeader className="flex gap-3">
          <IconPlayerPlay size={24} className="text-primary" />
          <div className="flex flex-col">
            <p className="text-lg font-semibold">Playback</p>
            <p className="text-small text-default-500">
              Configure video playback behavior
            </p>
          </div>
        </CardHeader>
        <Divider />
        <CardBody className="gap-6">
          {/* Sync Interval Setting */}
          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium">Watch Progress Sync Interval</p>
                <p className="text-small text-default-500">
                  How often to save your watch position to the database. Lower values
                  give more precise resume points but use more resources.
                </p>
              </div>
              <span className="text-lg font-mono text-primary min-w-[60px] text-right">
                {syncInterval}s
              </span>
            </div>
            <Slider
              aria-label="Sync interval in seconds"
              step={5}
              minValue={5}
              maxValue={60}
              value={syncInterval}
              onChange={(value) => setSyncInterval(value as number)}
              className="max-w-md"
              showSteps
              marks={[
                { value: 5, label: '5s' },
                { value: 15, label: '15s' },
                { value: 30, label: '30s' },
                { value: 60, label: '60s' },
              ]}
            />
          </div>

          <Divider />

          {/* Save Button */}
          <div className="flex justify-end">
            <Button
              color="primary"
              onPress={handleSave}
              isLoading={saving}
              isDisabled={!hasChanges}
              startContent={!saving && <IconDeviceFloppy size={18} />}
            >
              Save Changes
            </Button>
          </div>
        </CardBody>
      </Card>

      <Card>
        <CardHeader className="flex items-center justify-between">
          <div className="flex gap-3 items-center">
            <IconServer2 size={24} className="text-primary" />
            <div className="flex flex-col">
              <p className="text-lg font-semibold">System</p>
              <p className="text-small text-default-500">
                Runtime platform and network library availability
              </p>
            </div>
          </div>
          <Button
            size="sm"
            variant="flat"
            onPress={() => void loadHealth(true)}
            isLoading={isRefreshingHealth}
            startContent={!isRefreshingHealth && <IconRefresh size={16} />}
          >
            Refresh
          </Button>
        </CardHeader>
        <Divider />
        <CardBody className="gap-4">
          {isHealthLoading ? (
            <div className="py-6 flex justify-center">
              <Spinner />
            </div>
          ) : (
            <>
              <div className="text-sm">
                <span className="text-default-500">Status:</span>
                <span className="ml-2 font-medium">{health?.status ?? "unknown"}</span>
              </div>
              <div className="text-sm">
                <span className="text-default-500">Version:</span>
                <span className="ml-2 font-medium">{health?.version ?? "unknown"}</span>
              </div>
              <div className="text-sm">
                <span className="text-default-500">Platform:</span>
                <span className="ml-2 font-medium">{health?.platform ?? "unknown"}</span>
              </div>
              <Divider />
              <div className="space-y-2">
                <p className="text-sm font-medium">Network Library Paths</p>
                {(health?.network_paths?.length ?? 0) === 0 ? (
                  <p className="text-sm text-default-500">No persisted network paths</p>
                ) : (
                  health!.network_paths.map((p) => (
                    <div key={p.path} className="rounded-medium bg-content2 px-3 py-2">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-mono text-xs break-all">{p.path}</span>
                        <span
                          className={`text-xs font-semibold ${p.reachable ? 'text-success' : 'text-danger'}`}
                        >
                          {p.reachable ? 'Online' : 'Offline'}
                        </span>
                      </div>
                      {p.message && (
                        <p className="mt-1 text-xs text-default-500">{p.message}</p>
                      )}
                    </div>
                  ))
                )}
              </div>
            </>
          )}
        </CardBody>
      </Card>
    </div>
  )
}
