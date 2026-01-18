import { useState } from 'react'
import { Button } from '@heroui/button'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Input } from '@heroui/input'
import { Select, SelectItem } from '@heroui/select'
import { Switch } from '@heroui/switch'
import { Divider } from '@heroui/divider'
import { FolderBrowserInput } from '../FolderBrowserInput'
import { QUALITY_PRESETS, type QualitySettings } from '../settings'
import { LIBRARY_TYPES, type LibraryType, type PostDownloadAction, type CreateLibraryInput } from '../../lib/graphql'

export interface AddLibraryModalProps {
  isOpen: boolean
  onClose: () => void
  onAdd: (library: CreateLibraryInput) => Promise<void>
  isLoading: boolean
}

const DEFAULT_QUALITY_SETTINGS: QualitySettings = {
  allowedResolutions: [],
  allowedVideoCodecs: [],
  allowedAudioFormats: [],
  requireHdr: false,
  allowedHdrTypes: [],
  allowedSources: [],
  releaseGroupBlacklist: [],
  releaseGroupWhitelist: [],
}

export function AddLibraryModal({ isOpen, onClose, onAdd, isLoading }: AddLibraryModalProps) {
  const [name, setName] = useState('')
  const [path, setPath] = useState('')
  const [libraryType, setLibraryType] = useState<LibraryType>('TV')
  const [autoScan, setAutoScan] = useState(true)
  const [scanInterval, setScanInterval] = useState(60)
  const [watchForChanges, setWatchForChanges] = useState(false)
  const [postDownloadAction, setPostDownloadAction] =
    useState<PostDownloadAction>('COPY')
  const [organizeFiles, setOrganizeFiles] = useState(true)
  const [autoAddDiscovered, setAutoAddDiscovered] = useState(true)
  const [qualityPreset, setQualityPreset] = useState('Any Quality')
  const [qualitySettings, setQualitySettings] = useState<QualitySettings>(DEFAULT_QUALITY_SETTINGS)

  const handlePresetChange = (presetName: string) => {
    setQualityPreset(presetName)
    const preset = QUALITY_PRESETS.find(p => p.name === presetName)
    if (preset) {
      setQualitySettings({
        ...DEFAULT_QUALITY_SETTINGS,
        ...preset.settings,
      })
    }
  }

  const handleSubmit = async () => {
    if (!name || !path) return
    await onAdd({
      name,
      path,
      libraryType,
      autoScan,
      scanIntervalMinutes: scanInterval,
      watchForChanges,
      postDownloadAction,
      organizeFiles,
      autoAddDiscovered,
      // Quality settings from preset
      allowedResolutions: qualitySettings.allowedResolutions,
      allowedVideoCodecs: qualitySettings.allowedVideoCodecs,
      allowedAudioFormats: qualitySettings.allowedAudioFormats,
      requireHdr: qualitySettings.requireHdr,
      allowedHdrTypes: qualitySettings.allowedHdrTypes,
      allowedSources: qualitySettings.allowedSources,
      releaseGroupBlacklist: qualitySettings.releaseGroupBlacklist,
      releaseGroupWhitelist: qualitySettings.releaseGroupWhitelist,
    })
    // Reset form
    setName('')
    setPath('')
    setLibraryType('TV')
    setAutoScan(true)
    setScanInterval(60)
    setWatchForChanges(false)
    setPostDownloadAction('COPY')
    setOrganizeFiles(true)
    setAutoAddDiscovered(true)
    setQualityPreset('Any Quality')
    setQualitySettings(DEFAULT_QUALITY_SETTINGS)
    onClose()
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="xl">
      <ModalContent>
        <ModalHeader>Add Library</ModalHeader>
        <ModalBody>
          <div className="space-y-4">
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
              onChange={(e) => setLibraryType(e.target.value as LibraryType)}
            >
              {LIBRARY_TYPES.map((type) => (
                <SelectItem key={type.value} textValue={type.label}>
                  <div className="flex items-center gap-2">
                    <type.Icon className="w-4 h-4" />
                    {type.label}
                  </div>
                </SelectItem>
              ))}
            </Select>

            <Divider />

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

            <Divider />

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

            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium">Organize files</p>
                <p className="text-xs text-default-500">
                  Organize downloaded files into show/season folders
                </p>
              </div>
              <Switch isSelected={organizeFiles} onValueChange={setOrganizeFiles} />
            </div>

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

            <Divider />

            <Select
              label="Quality Preset"
              selectedKeys={[qualityPreset]}
              onChange={(e) => handlePresetChange(e.target.value)}
              description="Quick quality filter setup (can be customized later in settings)"
            >
              {QUALITY_PRESETS.map((preset) => (
                <SelectItem key={preset.name} textValue={preset.name}>
                  <div className="flex flex-col">
                    <span>{preset.name}</span>
                    <span className="text-xs text-default-400">
                      {preset.description}
                    </span>
                  </div>
                </SelectItem>
              ))}
            </Select>
          </div>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            onPress={handleSubmit}
            isDisabled={!name || !path}
            isLoading={isLoading}
          >
            Add Library
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
