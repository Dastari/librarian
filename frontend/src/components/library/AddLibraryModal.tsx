import { useState, useCallback } from 'react'
import { Button } from '@heroui/button'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { LibrarySettingsForm, DEFAULT_LIBRARY_SETTINGS, type LibrarySettingsValues } from './LibrarySettingsForm'
import type { CreateLibraryInput } from '../../lib/graphql'

export type CreateLibraryFormInput = Omit<
  CreateLibraryInput,
  'UserId' | 'CreatedAt' | 'UpdatedAt'
>

export interface AddLibraryModalProps {
  isOpen: boolean
  onClose: () => void
  onAdd: (library: CreateLibraryFormInput) => Promise<void>
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
      Name: values.name,
      Path: values.path,
      LibraryType: values.libraryType,
      AutoScan: values.autoScan,
      ScanIntervalMinutes: values.scanIntervalMinutes,
      WatchForChanges: values.watchForChanges,
      AutoAddDiscovered: values.autoAddDiscovered,
      AutoDownload: values.autoDownload,
      AutoHunt: values.autoHunt,
      Scanning: false,
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
    <Modal
      isOpen={isOpen}
      onClose={handleClose}
      size="xl"
      scrollBehavior="inside"
      aria-labelledby="add-library-modal-title"
      aria-describedby="add-library-modal-description"
    >
      <ModalContent>
        <ModalHeader id="add-library-modal-title">Add Library</ModalHeader>
        <ModalBody id="add-library-modal-description">
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
