import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { Button } from "@heroui/button";
import { addToast } from "@heroui/toast";
import { useState } from "react";
import { IconAlertTriangle } from "@tabler/icons-react";
import { sanitizeError } from "../../lib/format";
import { useMutation } from "../../lib/graphql/client";
import { DeleteMovieModalDocument } from "../../lib/graphql/generated/graphql";

interface DeleteMovieModalProps {
  isOpen: boolean;
  onClose: () => void;
  movie: { id: string; title: string } | null;
  onDeleted?: () => void;
}

export function DeleteMovieModal({
  isOpen,
  onClose,
  movie,
  onDeleted,
}: DeleteMovieModalProps) {
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteMovie] = useMutation(DeleteMovieModalDocument);

  const handleDelete = async () => {
    if (!movie) return;

    setIsDeleting(true);
    try {
      const { data, error } = await deleteMovie({
        variables: { Id: movie.id },
      });

      if (error || !data?.DeleteMovie?.Success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.DeleteMovie?.Error || "Failed to delete movie",
          ),
          color: "danger",
        });
        return;
      }

      addToast({
        title: "Deleted",
        description: `"${movie.title}" removed from library`,
        color: "success",
      });

      onClose();
      onDeleted?.();
    } catch (err) {
      console.error("Failed to delete movie:", err);
      addToast({
        title: "Error",
        description: sanitizeError(err),
        color: "danger",
      });
    } finally {
      setIsDeleting(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="md">
      <ModalContent>
        <ModalHeader className="flex gap-2 items-center">
          <IconAlertTriangle size={24} className="text-warning" />
          Delete Movie
        </ModalHeader>
        <ModalBody>
          <p className="text-default-600">
            Are you sure you want to delete{" "}
            <span className="font-semibold">"{movie?.title}"</span>?
          </p>
          <p className="text-small text-default-500">
            This will remove the movie from your library. Downloaded files will
            not be deleted.
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
