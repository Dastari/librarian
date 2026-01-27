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
import { IconRefresh } from "@tabler/icons-react";
import { graphqlClient, SCAN_LIBRARY_MUTATION } from "../../lib/graphql";

export interface ScanLibraryModalProps {
  isOpen: boolean;
  onClose: () => void;
  libraryId: string | null;
  libraryName: string | null;
  onScanStarted?: () => void;
}

/**
 * Reusable modal for scanning a library.
 * Pass libraryId and libraryName when opening.
 *
 * @example
 * ```tsx
 * const [scanTarget, setScanTarget] = useState<{id: string, name: string} | null>(null)
 * const { isOpen, onOpen, onClose } = useDisclosure()
 *
 * const handleScanClick = (id: string, name: string) => {
 *   setScanTarget({ id, name })
 *   onOpen()
 * }
 *
 * <ScanLibraryModal
 *   isOpen={isOpen}
 *   onClose={onClose}
 *   libraryId={scanTarget?.id ?? null}
 *   libraryName={scanTarget?.name ?? null}
 * />
 * ```
 */
export function ScanLibraryModal({
  isOpen,
  onClose,
  libraryId,
  libraryName,
  onScanStarted,
}: ScanLibraryModalProps) {
  const [isScanning, setIsScanning] = useState(false);

  const handleScan = async () => {
    if (!libraryId) return;

    try {
      setIsScanning(true);
      const { data, error } = await graphqlClient
        .mutation<{
          ScanLibrary: { Status: string; Message: string | null };
        }>(SCAN_LIBRARY_MUTATION, { Id: libraryId })
        .toPromise();

      if (error) {
        addToast({
          title: "Error",
          description: error.message || "Failed to start scan",
          color: "danger",
        });
        return;
      }

      addToast({
        title: "Scan Started",
        description: data?.ScanLibrary.Message || `Scanning ${libraryName}...`,
        color: "primary",
      });

      onScanStarted?.();
      onClose();
    } catch (err) {
      console.error("Failed to scan library:", err);
      addToast({
        title: "Error",
        description: "Failed to start scan",
        color: "danger",
      });
    } finally {
      setIsScanning(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="sm">
      <ModalContent>
        <ModalHeader className="flex items-center gap-2">
          <IconRefresh size={20} className="text-primary" />
          Scan Library
        </ModalHeader>
        <ModalBody>
          <p>
            Start a scan for <strong>"{libraryName}"</strong>?
          </p>
          <p className="text-default-500 text-sm mt-2">
            This will check for new files and update metadata for existing
            items.
          </p>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose} isDisabled={isScanning}>
            Cancel
          </Button>
          <Button color="primary" onPress={handleScan} isLoading={isScanning}>
            Start Scan
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
