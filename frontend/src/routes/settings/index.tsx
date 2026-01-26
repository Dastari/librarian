import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect } from 'react'
import { Card, CardBody, CardHeader } from '@heroui/card'
import { Slider } from '@heroui/slider'
import { Button } from '@heroui/button'
import { Spinner } from '@heroui/spinner'
import { Divider } from '@heroui/divider'
import { addToast } from '@heroui/toast'
import { IconPlayerPlay, IconDeviceFloppy } from '@tabler/icons-react'
import {
  graphqlClient,
  PlaybackSyncIntervalDocument,
  UpdateAppSettingDocument,
} from '../../lib/graphql'

export const Route = createFileRoute('/settings/')({
  component: GeneralSettingsPage,
})

const PLAYBACK_SYNC_KEY = 'playback_sync_interval'

function GeneralSettingsPage() {
  const [settingId, setSettingId] = useState<string | null>(null)
  const [savedSyncInterval, setSavedSyncInterval] = useState<number | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [syncInterval, setSyncInterval] = useState(15)

  // Load playback sync interval from app settings
  useEffect(() => {
    async function loadSettings() {
      try {
        const result = await graphqlClient
          .query(PlaybackSyncIntervalDocument, { Key: PLAYBACK_SYNC_KEY })
          .toPromise()

        const node = result.data?.AppSettings?.Edges?.[0]?.Node
        if (node) {
          setSettingId(node.Id)
          const val = Number(node.Value)
          if (Number.isFinite(val)) {
            setSyncInterval(val)
            setSavedSyncInterval(val)
          }
        }
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err)
        if (!errorMsg.toLowerCase().includes('authentication')) {
          console.error('Failed to load settings:', err)
          addToast({
            title: 'Error',
            description: 'Failed to load playback settings',
            color: 'danger',
          })
        }
      } finally {
        setLoading(false)
      }
    }

    loadSettings()
  }, [])

  const handleSave = async () => {
    if (settingId == null) return
    setSaving(true)
    try {
      const result = await graphqlClient
        .mutation(UpdateAppSettingDocument, {
          Id: settingId,
          Input: { Value: String(syncInterval) },
        })
        .toPromise()

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
    } finally {
      setSaving(false)
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
    </div>
  )
}
