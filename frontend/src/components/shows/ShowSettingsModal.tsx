import { useState, useEffect } from 'react'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'
import { Select, SelectItem } from '@heroui/select'
import { Divider } from '@heroui/divider'
import { QualitySettingsCard, type QualitySettings, DEFAULT_QUALITY_SETTINGS } from '../settings'
import type { TvShow, MonitorType } from '../../lib/graphql'

export interface ShowSettingsModalProps {
  isOpen: boolean
  onClose: () => void
  show: TvShow | null
  onSave: (settings: ShowSettingsInput) => Promise<void>
  isLoading: boolean
}

export interface ShowSettingsInput {
  monitorType?: MonitorType
  autoDownloadOverride: boolean | null
  autoHuntOverride: boolean | null
  organizeFilesOverride: boolean | null
  renameStyleOverride: string | null
  // Quality override settings
  allowedResolutionsOverride: string[] | null
  allowedVideoCodecsOverride: string[] | null
  allowedAudioFormatsOverride: string[] | null
  requireHdrOverride: boolean | null
  allowedHdrTypesOverride: string[] | null
  allowedSourcesOverride: string[] | null
  releaseGroupBlacklistOverride: string[] | null
  releaseGroupWhitelistOverride: string[] | null
}

type OverrideOption = 'inherit' | 'enabled' | 'disabled'
type RenameStyleOption = 'inherit' | 'none' | 'clean' | 'preserve_info'

export function ShowSettingsModal({
  isOpen,
  onClose,
  show,
  onSave,
  isLoading,
}: ShowSettingsModalProps) {
  // Automation settings
  const [monitorType, setMonitorType] = useState<MonitorType>('ALL')
  const [autoDownloadOverride, setAutoDownloadOverride] = useState<OverrideOption>('inherit')
  const [autoHuntOverride, setAutoHuntOverride] = useState<OverrideOption>('inherit')
  
  // Organization settings
  const [organizeFilesOverride, setOrganizeFilesOverride] = useState<OverrideOption>('inherit')
  const [renameStyleOverride, setRenameStyleOverride] = useState<RenameStyleOption>('inherit')
  
  // Quality settings
  const [isInheritingQuality, setIsInheritingQuality] = useState(true)
  const [qualitySettings, setQualitySettings] = useState<QualitySettings>(DEFAULT_QUALITY_SETTINGS)

  // Initialize form when show changes
  useEffect(() => {
    if (show) {
      // Monitor type
      setMonitorType(show.monitorType)
      
      // Automation
      setAutoDownloadOverride(
        show.autoDownloadOverride === null ? 'inherit' : show.autoDownloadOverride ? 'enabled' : 'disabled'
      )
      setAutoHuntOverride(
        show.autoHuntOverride === null ? 'inherit' : show.autoHuntOverride ? 'enabled' : 'disabled'
      )
      
      // Organization
      setOrganizeFilesOverride(
        show.organizeFilesOverride === null ? 'inherit' : show.organizeFilesOverride ? 'enabled' : 'disabled'
      )
      setRenameStyleOverride(
        show.renameStyleOverride === null ? 'inherit' : (show.renameStyleOverride as RenameStyleOption)
      )
      
      // Quality
      const qualityInherited = show.allowedResolutionsOverride === null
      setIsInheritingQuality(qualityInherited)
      if (!qualityInherited) {
        setQualitySettings({
          allowedResolutions: show.allowedResolutionsOverride || [],
          allowedVideoCodecs: show.allowedVideoCodecsOverride || [],
          allowedAudioFormats: show.allowedAudioFormatsOverride || [],
          requireHdr: show.requireHdrOverride || false,
          allowedHdrTypes: show.allowedHdrTypesOverride || [],
          allowedSources: show.allowedSourcesOverride || [],
          releaseGroupBlacklist: show.releaseGroupBlacklistOverride || [],
          releaseGroupWhitelist: show.releaseGroupWhitelistOverride || [],
        })
      } else {
        setQualitySettings(DEFAULT_QUALITY_SETTINGS)
      }
    }
  }, [show])

  const handleSave = async () => {
    await onSave({
      monitorType,
      autoDownloadOverride: autoDownloadOverride === 'inherit' ? null : autoDownloadOverride === 'enabled',
      autoHuntOverride: autoHuntOverride === 'inherit' ? null : autoHuntOverride === 'enabled',
      organizeFilesOverride: organizeFilesOverride === 'inherit' ? null : organizeFilesOverride === 'enabled',
      renameStyleOverride: renameStyleOverride === 'inherit' ? null : renameStyleOverride,
      // Quality overrides
      allowedResolutionsOverride: isInheritingQuality ? null : qualitySettings.allowedResolutions,
      allowedVideoCodecsOverride: isInheritingQuality ? null : qualitySettings.allowedVideoCodecs,
      allowedAudioFormatsOverride: isInheritingQuality ? null : qualitySettings.allowedAudioFormats,
      requireHdrOverride: isInheritingQuality ? null : qualitySettings.requireHdr,
      allowedHdrTypesOverride: isInheritingQuality ? null : qualitySettings.allowedHdrTypes,
      allowedSourcesOverride: isInheritingQuality ? null : qualitySettings.allowedSources,
      releaseGroupBlacklistOverride: isInheritingQuality ? null : qualitySettings.releaseGroupBlacklist,
      releaseGroupWhitelistOverride: isInheritingQuality ? null : qualitySettings.releaseGroupWhitelist,
    })
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="3xl" scrollBehavior="inside">
      <ModalContent>
        <ModalHeader>Show Settings</ModalHeader>
        <ModalBody className="gap-0">
          {/* Automation Section */}
          <div className="mb-4">
            <h4 className="text-sm font-semibold text-default-700 mb-1">Automation</h4>
            <p className="text-xs text-default-500 mb-3">Control how episodes are monitored and downloaded</p>
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
              <Select
                label="Monitoring"
                labelPlacement="outside"
                selectedKeys={[monitorType]}
                onSelectionChange={(keys) => {
                  const value = Array.from(keys)[0] as MonitorType
                  if (value) setMonitorType(value)
                }}
                size="sm"
                description="Which episodes to track"
              >
                <SelectItem key="ALL">All Episodes</SelectItem>
                <SelectItem key="FUTURE">Future Episodes Only</SelectItem>
                <SelectItem key="NONE">None</SelectItem>
              </Select>

              <Select
                label="Auto Download"
                labelPlacement="outside"
                selectedKeys={[autoDownloadOverride]}
                onSelectionChange={(keys) => {
                  const value = Array.from(keys)[0] as OverrideOption
                  if (value) setAutoDownloadOverride(value)
                }}
                size="sm"
                description="Download from RSS feeds"
              >
                <SelectItem key="inherit">Inherit from Library</SelectItem>
                <SelectItem key="enabled">Enabled</SelectItem>
                <SelectItem key="disabled">Disabled</SelectItem>
              </Select>

              <Select
                label="Auto Hunt"
                labelPlacement="outside"
                selectedKeys={[autoHuntOverride]}
                onSelectionChange={(keys) => {
                  const value = Array.from(keys)[0] as OverrideOption
                  if (value) setAutoHuntOverride(value)
                }}
                size="sm"
                description="Search indexers for missing"
              >
                <SelectItem key="inherit">Inherit from Library</SelectItem>
                <SelectItem key="enabled">Enabled</SelectItem>
                <SelectItem key="disabled">Disabled</SelectItem>
              </Select>
            </div>
          </div>

          <Divider className="my-4" />

          {/* Organization Section */}
          <div className="mb-4">
            <h4 className="text-sm font-semibold text-default-700 mb-1">Organization</h4>
            <p className="text-xs text-default-500 mb-3">Control how files are organized and named</p>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <Select
                label="Organize Files"
                labelPlacement="outside"
                selectedKeys={[organizeFilesOverride]}
                onSelectionChange={(keys) => {
                  const value = Array.from(keys)[0] as OverrideOption
                  if (value) setOrganizeFilesOverride(value)
                }}
                size="sm"
                description="Move into show/season folders"
              >
                <SelectItem key="inherit">Inherit from Library</SelectItem>
                <SelectItem key="enabled">Enabled</SelectItem>
                <SelectItem key="disabled">Disabled</SelectItem>
              </Select>

              <Select
                label="File Naming"
                labelPlacement="outside"
                selectedKeys={[renameStyleOverride]}
                onSelectionChange={(keys) => {
                  const value = Array.from(keys)[0] as RenameStyleOption
                  if (value) setRenameStyleOverride(value)
                }}
                size="sm"
                description="How to rename files"
              >
                <SelectItem key="inherit">Inherit from Library</SelectItem>
                <SelectItem key="none">Keep Original</SelectItem>
                <SelectItem key="clean">Clean Name</SelectItem>
                <SelectItem key="preserve_info">With Quality Info</SelectItem>
              </Select>
            </div>
          </div>

          <Divider className="my-4" />

          {/* Quality Filters Section */}
          <div>
            <h4 className="text-sm font-semibold text-default-700 mb-1">Quality Filters</h4>
            <p className="text-xs text-default-500 mb-3">Control which releases are accepted</p>
            <QualitySettingsCard
              settings={qualitySettings}
              onChange={setQualitySettings}
              isOverrideMode={true}
              isInheriting={isInheritingQuality}
              onInheritChange={setIsInheritingQuality}
              title=""
              noCard
            />
          </div>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button color="primary" onPress={handleSave} isLoading={isLoading}>
            Save Settings
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
