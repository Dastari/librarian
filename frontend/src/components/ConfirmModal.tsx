import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from '@heroui/modal'
import { Button } from '@heroui/button'

export interface ConfirmModalProps {
  isOpen: boolean
  onClose: () => void
  onConfirm: () => void
  title: string
  message: string
  description?: string
  confirmLabel?: string
  cancelLabel?: string
  confirmColor?: 'primary' | 'danger' | 'warning' | 'success' | 'default'
  isLoading?: boolean
}

/**
 * A reusable confirmation modal dialog.
 * Use with useDisclosure() hook for state management.
 * 
 * @example
 * ```tsx
 * const { isOpen, onOpen, onClose } = useDisclosure()
 * const [pendingAction, setPendingAction] = useState<() => void>()
 * 
 * const handleDelete = () => {
 *   setPendingAction(() => () => deleteItem(id))
 *   onOpen()
 * }
 * 
 * <ConfirmModal
 *   isOpen={isOpen}
 *   onClose={onClose}
 *   onConfirm={() => { pendingAction?.(); onClose() }}
 *   title="Delete Item"
 *   message="Are you sure you want to delete this item?"
 *   confirmLabel="Delete"
 *   confirmColor="danger"
 * />
 * ```
 */
export function ConfirmModal({
  isOpen,
  onClose,
  onConfirm,
  title,
  message,
  description,
  confirmLabel = 'Confirm',
  cancelLabel = 'Cancel',
  confirmColor = 'danger',
  isLoading = false,
}: ConfirmModalProps) {
  return (
    <Modal isOpen={isOpen} onClose={onClose} size="sm">
      <ModalContent>
        <ModalHeader>{title}</ModalHeader>
        <ModalBody>
          <p>{message}</p>
          {description && (
            <p className="text-default-500 text-sm mt-2">{description}</p>
          )}
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose} isDisabled={isLoading}>
            {cancelLabel}
          </Button>
          <Button color={confirmColor} onPress={onConfirm} isLoading={isLoading}>
            {confirmLabel}
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
