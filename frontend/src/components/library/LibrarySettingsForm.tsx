import { useState, useEffect, useCallback, useMemo } from 'react'
import { Input } from '@heroui/input'
import { Select, SelectItem } from '@heroui/select'
import { Switch } from '@heroui/switch'
import { Divider } from '@heroui/divider'
import { Accordion, AccordionItem } from '@heroui/accordion'
import { FolderBrowserInput } from '../FolderBrowserInput'
import { NamingPatternSelector } from './NamingPatternSelector'
import { QualitySettingsCard, QUALITY_PRESETS, type QualitySettings } from '../settings'
import { LIBRARY_TYPES, type LibraryType, type PostDownloadAction } from '../../lib/graphql'
import { IconFolder, IconRefresh, IconDownload, IconSettings, IconFilter } from '@tabler/icons-react'

// ============================================================================
// Types
// ============================================================================

export interface LibrarySettingsValues {
  name: string
  path: string
  libraryType: LibraryType
  autoScan: boolean
  scanIntervalMinutes: number
  watchForChanges: boolean
  postDownloadAction: PostDownloadAction
  organizeFiles: boolean
  namingPattern: string | null
  autoAddDiscovered: boolean
  autoDownload: boolean
  autoHunt: boolean
  // Quality settings
  allowedResolutions: string[]
  allowedVideoCodecs: string[]
  allowedAudioFormats: string[]
  requireHdr: boolean
  allowedHdrTypes: string[]
  allowedSources: string[]
  releaseGroupBlacklist: string[]
  releaseGroupWhitelist: string[]
}

export const DEFAULT_LIBRARY_SETTINGS: LibrarySettingsValues = {
  name: '',
  path: '',
  libraryType: 'TV',
  autoScan: true,
  scanIntervalMinutes: 60,
  watchForChanges: false,
  postDownloadAction: 'COPY',
  organizeFiles: true,
  namingPattern: null,
  autoAddDiscovered: true,
  autoDownload: true,
  autoHunt: false,
  allowedResolutions: [],
  allowedVideoCodecs: [],
  allowedAudioFormats: [],
  requireHdr: false,
  allowedHdrTypes: [],
  allowedSources: [],
  releaseGroupBlacklist: [],
  releaseGroupWhitelist: [],
}

export interface LibrarySettingsFormProps {
  /** Initial values for the form */
  initialValues?: Partial<LibrarySettingsValues>
  /** Called when any value changes */
  onChange: (values: LibrarySettingsValues) => void
  /** Mode determines which fields are shown/editable */
  mode: 'create' | 'edit'
  /** Whether to use Card wrappers (for settings page) or flat layout (for modal) */
  useCards?: boolean
  /** Whether to show quality preset selector (create mode) or full settings (edit mode) */
  qualityMode?: 'preset' | 'full'
}

// ============================================================================
// Reusable Setting Row Component
// ============================================================================

interface SettingRowProps {
  label: string
  description: string
  children: React.ReactNode
}

function SettingRow({ label, description, children }: SettingRowProps) {
  return (
    <div className="flex items-center justify-between">
      <div>
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-default-500">{description}</p>
      </div>
      {children}
    </div>
  )
}

// ============================================================================
// Main Component
// ============================================================================

export function LibrarySettingsForm({
  initialValues,
  onChange,
  mode,
  useCards = false,
  qualityMode = mode === 'create' ? 'preset' : 'full',
}: LibrarySettingsFormProps) {
  // Merge initial values with defaults
  const [values, setValues] = useState<LibrarySettingsValues>(() => ({
    ...DEFAULT_LIBRARY_SETTINGS,
    ...initialValues,
  }))
  
  // Quality preset (only used in create mode)
  const [qualityPreset, setQualityPreset] = useState('Any Quality')

  // Reset when initial values change
  useEffect(() => {
    setValues({
      ...DEFAULT_LIBRARY_SETTINGS,
      ...initialValues,
    })
  }, [initialValues])

  // Notify parent of changes
  const updateValue = useCallback(<K extends keyof LibrarySettingsValues>(
    key: K,
    value: LibrarySettingsValues[K]
  ) => {
    setValues(prev => {
      const next = { ...prev, [key]: value }
      onChange(next)
      return next
    })
  }, [onChange])

  // Quality settings as a single object
  const qualitySettings = useMemo<QualitySettings>(() => ({
    allowedResolutions: values.allowedResolutions,
    allowedVideoCodecs: values.allowedVideoCodecs,
    allowedAudioFormats: values.allowedAudioFormats,
    requireHdr: values.requireHdr,
    allowedHdrTypes: values.allowedHdrTypes,
    allowedSources: values.allowedSources,
    releaseGroupBlacklist: values.releaseGroupBlacklist,
    releaseGroupWhitelist: values.releaseGroupWhitelist,
  }), [values])

  const handleQualityChange = useCallback((settings: QualitySettings) => {
    setValues(prev => {
      const next = {
        ...prev,
        allowedResolutions: settings.allowedResolutions,
        allowedVideoCodecs: settings.allowedVideoCodecs,
        allowedAudioFormats: settings.allowedAudioFormats,
        requireHdr: settings.requireHdr,
        allowedHdrTypes: settings.allowedHdrTypes,
        allowedSources: settings.allowedSources,
        releaseGroupBlacklist: settings.releaseGroupBlacklist,
        releaseGroupWhitelist: settings.releaseGroupWhitelist,
      }
      onChange(next)
      return next
    })
  }, [onChange])

  const handlePresetChange = useCallback((presetName: string) => {
    setQualityPreset(presetName)
    const preset = QUALITY_PRESETS.find(p => p.name === presetName)
    if (preset) {
      handleQualityChange({
        ...DEFAULT_LIBRARY_SETTINGS,
        ...preset.settings,
      } as QualitySettings)
    }
  }, [handleQualityChange])

  // ============================================================================
  // Render Sections
  // ============================================================================

  const renderGeneralSection = () => (
    <>
      <Input
        label="Library Name"
        labelPlacement="inside"
        variant="flat"
        placeholder="e.g., Movies, TV Shows"
        value={values.name}
        onChange={(e) => updateValue('name', e.target.value)}
        classNames={{
          label: 'text-sm font-medium text-primary!',
        }}
      />

      <FolderBrowserInput
        label="Path"
        value={values.path}
        onChange={(v) => updateValue('path', v)}
        placeholder="/data/media/TV"
        description="Full path to the media folder"
        modalTitle="Select Library Folder"
      />

      <Select
        label="Library Type"
        selectedKeys={[values.libraryType]}
        onChange={(e) => updateValue('libraryType', e.target.value as LibraryType)}
        isDisabled={mode === 'edit'}
        description={mode === 'edit' ? 'Library type cannot be changed after creation' : undefined}
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
    </>
  )

  const renderScanningSection = () => (
    <>
      <SettingRow
        label="Auto-scan"
        description="Automatically scan for new files periodically"
      >
        <Switch
          isSelected={values.autoScan}
          onValueChange={(v) => updateValue('autoScan', v)}
        />
      </SettingRow>

      {values.autoScan && (
        <Input
          type="number"
          label="Scan Interval (minutes)"
          labelPlacement="inside"
          variant="flat"
          placeholder="60"
          description="How often to scan for new files (5-1440 minutes)"
          value={values.scanIntervalMinutes.toString()}
          onChange={(e) => updateValue('scanIntervalMinutes', parseInt(e.target.value) || 60)}
          min={5}
          max={1440}
          classNames={{
            label: 'text-sm font-medium text-primary!',
          }}
        />
      )}

      <Divider />

      <SettingRow
        label="Watch for changes"
        description="Use filesystem notifications for instant detection"
      >
        <Switch
          isSelected={values.watchForChanges}
          onValueChange={(v) => updateValue('watchForChanges', v)}
        />
      </SettingRow>
    </>
  )

  const renderAutomationSection = () => {
    const libraryType = values.libraryType
    if (libraryType !== 'TV' && libraryType !== 'MOVIES') return null

    return (
      <>
        <SettingRow
          label="Auto-download"
          description={
            libraryType === 'TV'
              ? 'Automatically download episodes from RSS feeds when they match'
              : 'Automatically download movies from RSS feeds when they match'
          }
        >
          <Switch
            isSelected={values.autoDownload}
            onValueChange={(v) => updateValue('autoDownload', v)}
          />
        </SettingRow>

        <Divider />

        <SettingRow
          label="Auto-hunt"
          description={
            libraryType === 'TV'
              ? 'Automatically search indexers for missing episodes'
              : 'Automatically search indexers for missing movies'
          }
        >
          <Switch
            isSelected={values.autoHunt}
            onValueChange={(v) => updateValue('autoHunt', v)}
          />
        </SettingRow>

        {libraryType === 'TV' && (
          <>
            <Divider />
            <SettingRow
              label="Auto-add discovered shows"
              description="Automatically add shows found during scanning"
            >
              <Switch
                isSelected={values.autoAddDiscovered}
                onValueChange={(v) => updateValue('autoAddDiscovered', v)}
              />
            </SettingRow>
          </>
        )}
      </>
    )
  }

  const renderPostDownloadSection = () => {
    const libraryType = values.libraryType
    const organizeDescription = {
      TV: 'Organize downloaded files into show/season folders',
      MOVIES: 'Organize downloaded files into movie folders',
      MUSIC: 'Organize downloaded files into artist/album folders',
      AUDIOBOOKS: 'Organize downloaded files into author/book folders',
      OTHER: 'Organize downloaded files into folders',
    }[libraryType] || 'Organize downloaded files into folders'

    return (
      <>
        <Select
          label="Post-download action"
          selectedKeys={[values.postDownloadAction]}
          onChange={(e) => updateValue('postDownloadAction', e.target.value as PostDownloadAction)}
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

        <SettingRow label="Organize files" description={organizeDescription}>
          <Switch
            isSelected={values.organizeFiles}
            onValueChange={(v) => updateValue('organizeFiles', v)}
          />
        </SettingRow>

        {values.organizeFiles && (
          <NamingPatternSelector
            value={values.namingPattern}
            onChange={(v) => updateValue('namingPattern', v)}
          />
        )}
      </>
    )
  }

  const renderQualitySection = () => {
    if (qualityMode === 'preset') {
      return (
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
      )
    }

    return (
      <QualitySettingsCard
        settings={qualitySettings}
        onChange={handleQualityChange}
        title="Quality Filters"
        description="Configure which releases to accept. Empty = accept any."
        noCard={!useCards}
      />
    )
  }

  // ============================================================================
  // Render with Accordions (for settings page)
  // ============================================================================

  if (useCards) {
    const showAutomation = values.libraryType === 'TV' || values.libraryType === 'MOVIES'

    // Build accordion items dynamically to avoid conditional children issue
    const accordionItems = [
      <AccordionItem
        key="general"
        aria-label="General"
        title={
          <div className="flex items-center gap-2">
            <IconFolder size={18} className="text-amber-400" />
            <span className="font-semibold">General</span>
          </div>
        }
        subtitle="Library name, path, and type"
      >
        <div className="space-y-4 pb-2">
          {renderGeneralSection()}
        </div>
      </AccordionItem>,

      <AccordionItem
        key="scanning"
        aria-label="Scanning"
        title={
          <div className="flex items-center gap-2">
            <IconRefresh size={18} className="text-blue-400" />
            <span className="font-semibold">Scanning</span>
          </div>
        }
        subtitle="How the library detects new files"
      >
        <div className="space-y-4 pb-2">
          {renderScanningSection()}
        </div>
      </AccordionItem>,

      <AccordionItem
        key="quality"
        aria-label="Quality Filters"
        title={
          <div className="flex items-center gap-2">
            <IconFilter size={18} className="text-warning" />
            <span className="font-semibold">Quality Filters</span>
          </div>
        }
        subtitle="Control which releases are accepted"
      >
        <div className="pb-2">
          <QualitySettingsCard
            settings={qualitySettings}
            onChange={handleQualityChange}
            title=""
            description="Configure which releases to accept. Empty = accept any."
            noCard
          />
        </div>
      </AccordionItem>,
    ]

    // Add automation section only for TV and Movies
    if (showAutomation) {
      accordionItems.push(
        <AccordionItem
          key="automation"
          aria-label="Automation"
          title={
            <div className="flex items-center gap-2">
              <IconDownload size={18} className="text-primary" />
              <span className="font-semibold">Automation</span>
            </div>
          }
          subtitle="Auto-download and hunting settings"
        >
          <div className="space-y-4 pb-2">
            {renderAutomationSection()}
          </div>
        </AccordionItem>
      )
    }

    // Add organization section
    accordionItems.push(
      <AccordionItem
        key="organization"
        aria-label="Organization"
        title={
          <div className="flex items-center gap-2">
            <IconSettings size={18} className="text-secondary" />
            <span className="font-semibold">Organization</span>
          </div>
        }
        subtitle="Post-download file handling"
      >
        <div className="space-y-4 pb-2">
          {renderPostDownloadSection()}
        </div>
      </AccordionItem>
    )

    return (
      <Accordion 
        selectionMode="multiple" 
        // defaultExpandedKeys={defaultKeys}
        variant="splitted"
      >
        {accordionItems}
      </Accordion>
    )
  }

  // ============================================================================
  // Render Flat (for modals)
  // ============================================================================

  return (
    <div className="space-y-4">
      {renderGeneralSection()}

      <Divider />

      {renderScanningSection()}

      <Divider />

      {renderPostDownloadSection()}

      {mode === 'create' && (values.libraryType === 'TV' || values.libraryType === 'MOVIES') && (
        <>
          <SettingRow
            label="Auto-add discovered shows"
            description="Automatically add shows found during scanning"
          >
            <Switch
              isSelected={values.autoAddDiscovered}
              onValueChange={(v) => updateValue('autoAddDiscovered', v)}
            />
          </SettingRow>
        </>
      )}

      <Divider />

      {renderQualitySection()}
    </div>
  )
}
