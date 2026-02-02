import { useState, useCallback } from 'react'
import { Button } from '@heroui/button'
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import {
  LibrarySettingsForm,
  DEFAULT_LIBRARY_SETTINGS,
  type LibrarySettingsFormValues,
} from "./LibrarySettingsForm";
import type { CreateLibraryInput } from '../../lib/graphql'

export type CreateLibraryFormInput = Omit<
  CreateLibraryInput,
  "UserId" | "CreatedAt" | "UpdatedAt" | "Scanning" | "LastScannedAt"
>;

export interface AddLibraryModalProps {
  isOpen: boolean
  onClose: () => void
  onAdd: (library: CreateLibraryFormInput) => Promise<void>
  isLoading: boolean
}

export function AddLibraryModal({
  isOpen,
  onClose,
  onAdd,
  isLoading,
}: AddLibraryModalProps) {
  const [formValues, setFormValues] = useState<LibrarySettingsFormValues>(
    DEFAULT_LIBRARY_SETTINGS,
  );
  const [isFormValid, setIsFormValid] = useState(false);

  const handleChange = useCallback(
    (values: LibrarySettingsFormValues, isValid: boolean) => {
      setFormValues(values);
      setIsFormValid(isValid);
    },
    [],
  );

  const handleSubmit = async () => {
    if (!isFormValid) return;

    await onAdd({
      Name: formValues.Name,
      Path: formValues.Path,
      LibraryType: formValues.LibraryType,
      AutoScan: formValues.AutoScan,
      ScanIntervalMinutes: formValues.ScanIntervalMinutes,
      WatchForChanges: formValues.WatchForChanges,
      AutoOrganize: formValues.AutoOrganize,
      NamingPattern: formValues.NamingPattern ?? "",
    });

    // Reset form
    setFormValues(DEFAULT_LIBRARY_SETTINGS);
    onClose();
  };

  const handleClose = () => {
    setFormValues(DEFAULT_LIBRARY_SETTINGS);
    onClose();
  };

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
          />
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={handleClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            onPress={handleSubmit}
            isDisabled={!isFormValid}
            isLoading={isLoading}
          >
            Add Library
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
