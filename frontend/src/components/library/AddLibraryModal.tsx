import { useState, useCallback } from 'react'
import { Button } from '@heroui/button'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { LibrarySettingsForm, DEFAULT_LIBRARY_SETTINGS, type LibrarySettingsValues } from './LibrarySettingsForm'
import type { CreateLibraryInput } from '../../lib/graphql'

export interface AddLibraryModalProps {
  isOpen: boolean
  onClose: () => void
  onAdd: (library: CreateLibraryInput) => Promise<void>
  isLoading: boolean
}

export function AddLibraryModal({ isOpen, onClose, onAdd, isLoading }: AddLibraryModalProps) {
  const [values, setValues] = useState<LibrarySettingsValues>(DEFAULT_LIBRARY_SETTINGS)

  const handleChange = useCallback((newValues: LibrarySettingsValues) => {
    setValues(newValues)
  }, [])

  const handleSubmit = async () => {
    if (!values.name || !values.path) return
    
    await onAdd({
      name: values.name,
      path: values.path,
      libraryType: values.libraryType,
      autoScan: values.autoScan,
      scanIntervalMinutes: values.scanIntervalMinutes,
      watchForChanges: values.watchForChanges,
      organizeFiles: values.organizeFiles,
      namingPattern: values.namingPattern || undefined,
      autoAddDiscovered: values.autoAddDiscovered,
      autoDownload: values.autoDownload,
      autoHunt: values.autoHunt,
      // Quality settings
      allowedResolutions: values.allowedResolutions,
      allowedVideoCodecs: values.allowedVideoCodecs,
      allowedAudioFormats: values.allowedAudioFormats,
      requireHdr: values.requireHdr,
      allowedHdrTypes: values.allowedHdrTypes,
      allowedSources: values.allowedSources,
      releaseGroupBlacklist: values.releaseGroupBlacklist,
      releaseGroupWhitelist: values.releaseGroupWhitelist,
    })
    
    // Reset form
    setValues(DEFAULT_LIBRARY_SETTINGS)
    onClose()
  }

  const handleClose = () => {
    setValues(DEFAULT_LIBRARY_SETTINGS)
    onClose()
  }

  return (
    <Modal isOpen={isOpen} onClose={handleClose} size="xl" scrollBehavior="inside">
      <ModalContent>
        <ModalHeader>Add Library</ModalHeader>
        <ModalBody>
          <LibrarySettingsForm
            initialValues={DEFAULT_LIBRARY_SETTINGS}
            onChange={handleChange}
            mode="create"
            useCards={false}
            qualityMode="preset"
          />
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={handleClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            onPress={handleSubmit}
            isDisabled={!values.name || !values.path}
            isLoading={isLoading}
          >
            Add Library
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
