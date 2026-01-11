import { useState, useEffect } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody, CardHeader } from '@heroui/card'
import { Input } from '@heroui/input'
import { Select, SelectItem } from '@heroui/select'
import { Switch } from '@heroui/switch'
import { Divider } from '@heroui/divider'
import { addToast } from '@heroui/toast'
import { FolderBrowserInput } from '../FolderBrowserInput'
import type { Library, LibraryType, PostDownloadAction, UpdateLibraryInput, QualityProfile } from '../../lib/graphql'

const LIBRARY_TYPES = [
  { value: 'MOVIES', label: 'Movies', icon: '🎬', color: 'purple' },
  { value: 'TV', label: 'TV Shows', icon: '📺', color: 'blue' },
  { value: 'MUSIC', label: 'Music', icon: '🎵', color: 'green' },
  { value: 'AUDIOBOOKS', label: 'Audiobooks', icon: '🎧', color: 'orange' },
  { value: 'OTHER', label: 'Other', icon: '📁', color: 'slate' },
] as const

interface LibrarySettingsTabProps {
  library: Library
  qualityProfiles: QualityProfile[]
  onSave: (input: UpdateLibraryInput) => Promise<void>
  isLoading: boolean
}

export function LibrarySettingsTab({ library, qualityProfiles, onSave, isLoading }: LibrarySettingsTabProps) {
  const [name, setName] = useState(library.name)
  const [path, setPath] = useState(library.path)
  const [libraryType] = useState<LibraryType>(library.libraryType)
  const [autoScan, setAutoScan] = useState(library.autoScan)
  const [scanInterval, setScanInterval] = useState(library.scanIntervalMinutes)
  const [watchForChanges, setWatchForChanges] = useState(library.watchForChanges)
  const [postDownloadAction, setPostDownloadAction] = useState<PostDownloadAction>(library.postDownloadAction)
  const [organizeFiles, setOrganizeFiles] = useState(library.organizeFiles)
  const [autoAddDiscovered, setAutoAddDiscovered] = useState(library.autoAddDiscovered)
  const [autoDownload, setAutoDownload] = useState(library.autoDownload)
  const [autoHunt, setAutoHunt] = useState(library.autoHunt)
  const [defaultQualityProfileId, setDefaultQualityProfileId] = useState<string | null>(library.defaultQualityProfileId)
  const [hasChanges, setHasChanges] = useState(false)

  // Reset form when library changes
  useEffect(() => {
    setName(library.name)
    setPath(library.path)
    setAutoScan(library.autoScan)
    setScanInterval(library.scanIntervalMinutes)
    setWatchForChanges(library.watchForChanges)
    setPostDownloadAction(library.postDownloadAction)
    setOrganizeFiles(library.organizeFiles)
    setAutoAddDiscovered(library.autoAddDiscovered)
    setAutoDownload(library.autoDownload)
    setAutoHunt(library.autoHunt)
    setDefaultQualityProfileId(library.defaultQualityProfileId)
    setHasChanges(false)
  }, [library])

  // Track changes
  useEffect(() => {
    const changed =
      name !== library.name ||
      path !== library.path ||
      autoScan !== library.autoScan ||
      scanInterval !== library.scanIntervalMinutes ||
      watchForChanges !== library.watchForChanges ||
      postDownloadAction !== library.postDownloadAction ||
      organizeFiles !== library.organizeFiles ||
      autoAddDiscovered !== library.autoAddDiscovered ||
      autoDownload !== library.autoDownload ||
      autoHunt !== library.autoHunt ||
      defaultQualityProfileId !== library.defaultQualityProfileId
    setHasChanges(changed)
  }, [name, path, autoScan, scanInterval, watchForChanges, postDownloadAction, organizeFiles, autoAddDiscovered, autoDownload, autoHunt, defaultQualityProfileId, library])

  const handleSubmit = async () => {
    if (!name || !path) {
      addToast({
        title: 'Validation Error',
        description: 'Name and path are required',
        color: 'danger',
      })
      return
    }

    await onSave({
      name,
      path,
      autoScan,
      scanIntervalMinutes: scanInterval,
      watchForChanges,
      postDownloadAction,
      organizeFiles,
      autoAddDiscovered,
      autoDownload,
      autoHunt,
      defaultQualityProfileId,
    })
  }

  const handleReset = () => {
    setName(library.name)
    setPath(library.path)
    setAutoScan(library.autoScan)
    setScanInterval(library.scanIntervalMinutes)
    setWatchForChanges(library.watchForChanges)
    setPostDownloadAction(library.postDownloadAction)
    setOrganizeFiles(library.organizeFiles)
    setAutoAddDiscovered(library.autoAddDiscovered)
    setAutoDownload(library.autoDownload)
    setAutoHunt(library.autoHunt)
    setDefaultQualityProfileId(library.defaultQualityProfileId)
  }


  return (
    <div className="space-y-6 grow">
      <div>
        <h2 className="text-xl font-semibold">Library Settings</h2>
        <p className="text-sm text-default-500">
          Configure how this library behaves
        </p>
      </div>

      {/* General Settings */}
      <Card>
        <CardHeader>
          <h3 className="font-semibold">General</h3>
        </CardHeader>
        <CardBody className="space-y-4">
          <Input
            label="Library Name"
            placeholder="e.g., Movies, TV Shows"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />

          <FolderBrowserInput
            label="Path"
            value={path}
            onChange={setPath}
            placeholder="/data/media/TV"
            description="Full path to the media folder"
            modalTitle="Select Library Folder"
          />

          <Select
            label="Library Type"
            selectedKeys={[libraryType]}
            isDisabled
            description="Library type cannot be changed after creation"
          >
            {LIBRARY_TYPES.map((type) => (
              <SelectItem key={type.value} textValue={type.label}>
                <span className="mr-2">{type.icon}</span>
                {type.label}
              </SelectItem>
            ))}
          </Select>
        </CardBody>
      </Card>

      {/* Scanning Settings */}
      <Card>
        <CardHeader>
          <h3 className="font-semibold">Scanning</h3>
        </CardHeader>
        <CardBody className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Auto-scan</p>
              <p className="text-xs text-default-500">
                Automatically scan for new files periodically
              </p>
            </div>
            <Switch isSelected={autoScan} onValueChange={setAutoScan} />
          </div>

          {autoScan && (
            <Input
              type="number"
              label="Scan Interval (minutes)"
              value={scanInterval.toString()}
              onChange={(e) => setScanInterval(parseInt(e.target.value) || 60)}
              min={5}
              max={1440}
            />
          )}

          <Divider />

          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Watch for changes</p>
              <p className="text-xs text-default-500">
                Use filesystem notifications for instant detection
              </p>
            </div>
            <Switch
              isSelected={watchForChanges}
              onValueChange={setWatchForChanges}
            />
          </div>
        </CardBody>
      </Card>

      {/* Quality Settings */}
      <Card>
        <CardHeader>
          <h3 className="font-semibold">Quality</h3>
        </CardHeader>
        <CardBody className="space-y-4">
          <Select
            label="Default Quality Profile"
            selectedKeys={defaultQualityProfileId ? [defaultQualityProfileId] : []}
            onChange={(e) =>
              setDefaultQualityProfileId(e.target.value || null)
            }
            description="Quality profile used for new shows added to this library"
            placeholder="Select a quality profile"
          >
            {qualityProfiles.map((profile) => (
              <SelectItem key={profile.id} textValue={profile.name}>
                <div className="flex flex-col">
                  <span>{profile.name}</span>
                  <span className="text-xs text-default-400">
                    {[
                      profile.preferredResolution,
                      profile.preferredCodec,
                      profile.requireHdr && 'HDR',
                    ].filter(Boolean).join(' • ') || 'Any quality'}
                  </span>
                </div>
              </SelectItem>
            ))}
          </Select>
        </CardBody>
      </Card>

      {/* Automation Settings */}
      <Card>
        <CardHeader>
          <h3 className="font-semibold">Automation</h3>
        </CardHeader>
        <CardBody className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Auto-download</p>
              <p className="text-xs text-default-500">
                Automatically download episodes from RSS feeds when they match
              </p>
            </div>
            <Switch isSelected={autoDownload} onValueChange={setAutoDownload} />
          </div>

          <Divider />

          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Auto-hunt</p>
              <p className="text-xs text-default-500">
                Automatically search indexers for missing episodes
              </p>
            </div>
            <Switch isSelected={autoHunt} onValueChange={setAutoHunt} />
          </div>

          <Divider />

          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Auto-add discovered shows</p>
              <p className="text-xs text-default-500">
                Automatically add shows found during scanning
              </p>
            </div>
            <Switch
              isSelected={autoAddDiscovered}
              onValueChange={setAutoAddDiscovered}
            />
          </div>
        </CardBody>
      </Card>

      {/* Post-download Settings */}
      <Card>
        <CardHeader>
          <h3 className="font-semibold">Post-download Behavior</h3>
        </CardHeader>
        <CardBody className="space-y-4">
          <Select
            label="Post-download action"
            selectedKeys={[postDownloadAction]}
            onChange={(e) =>
              setPostDownloadAction(e.target.value as PostDownloadAction)
            }
            description="What to do with files after downloading"
          >
            <SelectItem key="COPY" textValue="Copy">
              Copy (preserves seeding)
            </SelectItem>
            <SelectItem key="MOVE" textValue="Move">
              Move (stops seeding)
            </SelectItem>
            <SelectItem key="HARDLINK" textValue="Hardlink">
              Hardlink (same disk only)
            </SelectItem>
          </Select>

          <Divider />

          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Organize files</p>
              <p className="text-xs text-default-500">
                Organize downloaded files into show/season folders
              </p>
            </div>
            <Switch isSelected={organizeFiles} onValueChange={setOrganizeFiles} />
          </div>
        </CardBody>
      </Card>

      {/* Save/Reset buttons */}
      <div className="flex items-center gap-3 ">
        <Button
          color="primary"
          onPress={handleSubmit}
          isDisabled={!hasChanges || !name || !path}
          isLoading={isLoading}
        >
          Save Changes
        </Button>
        <Button
          variant="flat"
          onPress={handleReset}
          isDisabled={!hasChanges}
        >
          Reset
        </Button>
        {hasChanges && (
          <span className="text-sm text-warning">You have unsaved changes</span>
        )}
      </div>
    </div>
  )
}
