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
import { IconAlertTriangle, IconRefresh } from "@tabler/icons-react";
import {
  MetadataAppSettingsDocument,
  ScanLibraryDocument,
  type MetadataAppSettingsQuery,
  type ScanLibraryMutation,
  type ScanLibraryMutationVariables,
} from "../../lib/graphql/generated/graphql";
import { useMutation, useQuery } from "../../lib/graphql/client";

export interface ScanLibraryModalProps {
  isOpen: boolean;
  onClose: () => void;
  libraryId: string | null;
  libraryName: string | null;
  libraryType?: string | null;
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
  libraryType,
  onScanStarted,
}: ScanLibraryModalProps) {
  const [isScanning, setIsScanning] = useState(false);
  const [scanLibrary] = useMutation<
    ScanLibraryMutation,
    ScanLibraryMutationVariables
  >(ScanLibraryDocument);
  const { data: metadataSettings } = useQuery<MetadataAppSettingsQuery>(
    MetadataAppSettingsDocument,
    { skip: !isOpen || libraryType?.toLowerCase() !== "movies" },
  );
  const tmdbReady =
    libraryType?.toLowerCase() !== "movies" ||
    metadataSettings?.appSettings.edges.some(
      ({ node }) =>
        node.key === "metadata.tmdb_api_key" &&
        node.value.trim() !== "" &&
        node.value !== "null",
    );

  const handleScan = async () => {
    if (!libraryId) return;

    try {
      setIsScanning(true);
      const { data, error } = await scanLibrary({
        variables: { id: libraryId },
      });

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
        description: data?.scanLibrary.message || `Scanning ${libraryName}...`,
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
          {!tmdbReady && (
            <div className="flex gap-2 rounded-medium border border-warning-300/50 bg-warning-50/10 p-3 text-sm text-warning-600">
              <IconAlertTriangle className="mt-0.5 shrink-0" size={18} />
              <div>
                <p className="font-medium">TMDB is not configured</p>
                <p>
                  Files will be discovered and preserved as unmatched, but new
                  Movie records cannot be created until a tested TMDB key is
                  saved in Metadata settings.
                </p>
              </div>
            </div>
          )}
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
