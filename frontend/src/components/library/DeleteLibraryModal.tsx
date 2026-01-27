import { useState } from "react";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { Button } from "@heroui/button";
import { addToast } from "@heroui/toast";
import { IconTrash } from "@tabler/icons-react";
import { graphqlClient } from "../../lib/graphql";
import { DeleteLibraryDocument } from "../../lib/graphql/generated/graphql";

export interface DeleteLibraryModalProps {
  isOpen: boolean;
  onClose: () => void;
  libraryId: string | null;
  libraryName: string | null;
  onDeleted?: () => void;
}

/**
 * Reusable modal for deleting a library.
 * Pass libraryId and libraryName when opening.
 *
 * @example
 * ```tsx
 * const [deleteTarget, setDeleteTarget] = useState<{id: string, name: string} | null>(null)
 * const { isOpen, onOpen, onClose } = useDisclosure()
 *
 * const handleDeleteClick = (id: string, name: string) => {
 *   setDeleteTarget({ id, name })
 *   onOpen()
 * }
 *
 * <DeleteLibraryModal
 *   isOpen={isOpen}
 *   onClose={onClose}
 *   libraryId={deleteTarget?.id ?? null}
 *   libraryName={deleteTarget?.name ?? null}
 *   onDeleted={() => refetchLibraries()}
 * />
 * ```
 */
export function DeleteLibraryModal({
  isOpen,
  onClose,
  libraryId,
  libraryName,
  onDeleted,
}: DeleteLibraryModalProps) {
  const [isDeleting, setIsDeleting] = useState(false);

  const handleDelete = async () => {
    if (!libraryId) return;

    try {
      setIsDeleting(true);
      const { data, error } = await graphqlClient
        .mutation(DeleteLibraryDocument, { Id: libraryId })
        .toPromise();

      if (error || !data?.DeleteLibrary.Success) {
        addToast({
          title: "Error",
          description:
            data?.DeleteLibrary.Error ||
            error?.message ||
            "Failed to delete library",
          color: "danger",
        });
        return;
      }

      addToast({
        title: "Deleted",
        description: `Library "${libraryName}" deleted`,
        color: "success",
      });

      onDeleted?.();
      onClose();
    } catch (err) {
      console.error("Failed to delete library:", err);
      addToast({
        title: "Error",
        description: "Failed to delete library",
        color: "danger",
      });
    } finally {
      setIsDeleting(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="sm">
      <ModalContent>
        <ModalHeader className="flex items-center gap-2">
          <IconTrash size={20} className="text-danger" />
          Delete Library
        </ModalHeader>
        <ModalBody>
          <p>
            Are you sure you want to delete <strong>"{libraryName}"</strong>?
          </p>
          <p className="text-default-500 text-sm mt-2">
            This will remove the library and all associated shows/movies from
            your collection. Downloaded files will not be deleted.
          </p>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose} isDisabled={isDeleting}>
            Cancel
          </Button>
          <Button color="danger" onPress={handleDelete} isLoading={isDeleting}>
            Delete
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
