import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'

export interface DeleteShowModalProps {
  isOpen: boolean
  onClose: () => void
  showName: string
  onConfirm: () => void
  isLoading: boolean
}

export function DeleteShowModal({
  isOpen,
  onClose,
  showName,
  onConfirm,
  isLoading,
}: DeleteShowModalProps) {
  return (
    <Modal isOpen={isOpen} onClose={onClose}>
      <ModalContent>
        <ModalHeader>Delete Show</ModalHeader>
        <ModalBody>
          <p>
            Are you sure you want to delete <strong>{showName}</strong>?
          </p>
          <p className="text-default-500 text-sm mt-2">
            This will remove the show from your library. Downloaded files will not be deleted.
          </p>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button color="danger" onPress={onConfirm} isLoading={isLoading}>
            Delete Show
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
