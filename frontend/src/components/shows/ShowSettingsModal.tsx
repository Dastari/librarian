import { useState, useEffect } from 'react'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'
import { Card, CardHeader, CardBody } from '@heroui/card'
import { Select, SelectItem } from '@heroui/select'
import { Divider } from '@heroui/divider'
import type { TvShow } from '../../lib/graphql'

export interface ShowSettingsModalProps {
  isOpen: boolean
  onClose: () => void
  show: TvShow | null
  onSave: (settings: ShowSettingsInput) => Promise<void>
  isLoading: boolean
}

export interface ShowSettingsInput {
  autoDownloadOverride: boolean | null
  organizeFilesOverride: boolean | null
  renameStyleOverride: string | null
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
  const [autoDownloadOverride, setAutoDownloadOverride] = useState<OverrideOption>('inherit')
  const [organizeFilesOverride, setOrganizeFilesOverride] = useState<OverrideOption>('inherit')
  const [renameStyleOverride, setRenameStyleOverride] = useState<RenameStyleOption>('inherit')

  // Initialize form when show changes
  useEffect(() => {
    if (show) {
      setAutoDownloadOverride(
        show.autoDownloadOverride === null ? 'inherit' : show.autoDownloadOverride ? 'enabled' : 'disabled'
      )
      setOrganizeFilesOverride(
        show.organizeFilesOverride === null ? 'inherit' : show.organizeFilesOverride ? 'enabled' : 'disabled'
      )
      setRenameStyleOverride(
        show.renameStyleOverride === null ? 'inherit' : (show.renameStyleOverride as RenameStyleOption)
      )
    }
  }, [show])

  const handleSave = async () => {
    await onSave({
      autoDownloadOverride: autoDownloadOverride === 'inherit' ? null : autoDownloadOverride === 'enabled',
      organizeFilesOverride: organizeFilesOverride === 'inherit' ? null : organizeFilesOverride === 'enabled',
      renameStyleOverride: renameStyleOverride === 'inherit' ? null : renameStyleOverride,
    })
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="lg">
      <ModalContent>
        <ModalHeader>Show Settings</ModalHeader>
        <ModalBody>
          <Card className="bg-content2">
            <CardHeader className="pb-0">
              <div className="flex flex-col">
                <p className="text-lg font-semibold">Download Settings</p>
                <p className="text-small text-default-500">Override library defaults for this show</p>
              </div>
            </CardHeader>
            <CardBody className="gap-4">
              <div>
                <p className="text-sm font-medium mb-2">Auto Download</p>
                <p className="text-xs text-default-400 mb-2">
                  Automatically download episodes when found in RSS feeds
                </p>
                <Select
                  selectedKeys={[autoDownloadOverride]}
                  onSelectionChange={(keys) => {
                    const value = Array.from(keys)[0] as OverrideOption
                    setAutoDownloadOverride(value)
                  }}
                  size="sm"
                  className="max-w-xs"
                >
                  <SelectItem key="inherit">Inherit from Library</SelectItem>
                  <SelectItem key="enabled">Enabled</SelectItem>
                  <SelectItem key="disabled">Disabled</SelectItem>
                </Select>
              </div>
            </CardBody>
          </Card>

          <Card className="bg-content2">
            <CardHeader className="pb-0">
              <div className="flex flex-col">
                <p className="text-lg font-semibold">File Organization</p>
                <p className="text-small text-default-500">Control how files are organized for this show</p>
              </div>
            </CardHeader>
            <CardBody className="gap-4">
              <div>
                <p className="text-sm font-medium mb-2">Organize Files</p>
                <p className="text-xs text-default-400 mb-2">
                  Automatically organize files into Show/Season folders
                </p>
                <Select
                  selectedKeys={[organizeFilesOverride]}
                  onSelectionChange={(keys) => {
                    const value = Array.from(keys)[0] as OverrideOption
                    setOrganizeFilesOverride(value)
                  }}
                  size="sm"
                  className="max-w-xs"
                >
                  <SelectItem key="inherit">Inherit from Library</SelectItem>
                  <SelectItem key="enabled">Enabled</SelectItem>
                  <SelectItem key="disabled">Disabled</SelectItem>
                </Select>
              </div>

              <Divider />

              <div>
                <p className="text-sm font-medium mb-2">File Naming</p>
                <p className="text-xs text-default-400 mb-2">
                  How to rename files after download
                </p>
                <Select
                  selectedKeys={[renameStyleOverride]}
                  onSelectionChange={(keys) => {
                    const value = Array.from(keys)[0] as RenameStyleOption
                    setRenameStyleOverride(value)
                  }}
                  size="sm"
                  className="max-w-xs"
                >
                  <SelectItem key="inherit">Inherit from Library</SelectItem>
                  <SelectItem key="none">Keep Original Filename</SelectItem>
                  <SelectItem key="clean">Clean (Show - S01E01 - Title)</SelectItem>
                  <SelectItem key="preserve_info">With Quality Info (Show - S01E01 [1080p])</SelectItem>
                </Select>
              </div>
            </CardBody>
          </Card>
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
