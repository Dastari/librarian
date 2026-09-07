import { useState, useCallback, useEffect } from "react";
import { Button } from "@heroui/button";
import { addToast } from "@heroui/toast";
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
import {
  configureNetworkPath,
  getFilesystemRuntimeInfo,
  type RuntimeFilesystemInfo,
} from "../../lib/graphql";
import type { CreateLibraryInput } from "../../lib/graphql/generated/graphql";

export type CreateLibraryFormInput = Omit<
  CreateLibraryInput,
  "userId" | "createdAt" | "updatedAt" | "scanning" | "lastScannedAt"
>;

export interface AddLibraryModalProps {
  isOpen: boolean;
  onClose: () => void;
  onAdd: (library: CreateLibraryFormInput) => Promise<void>;
  isLoading: boolean;
}

export function AddLibraryModal({
  isOpen,
  onClose,
  onAdd,
  isLoading,
}: AddLibraryModalProps) {
  const [closeSignal, setCloseSignal] = useState(0);
  const [formValues, setFormValues] = useState<LibrarySettingsFormValues>(
    DEFAULT_LIBRARY_SETTINGS,
  );
  const [isFormValid, setIsFormValid] = useState(false);
  const [runtimeInfo, setRuntimeInfo] = useState<RuntimeFilesystemInfo | null>(
    null,
  );

  useEffect(() => {
    if (!isOpen) return;
    let active = true;
    getFilesystemRuntimeInfo()
      .then((info) => {
        if (active) setRuntimeInfo(info);
      })
      .catch(() => {
        if (active) setRuntimeInfo(null);
      });

    return () => {
      active = false;
    };
  }, [isOpen]);

  const handleChange = useCallback(
    (values: LibrarySettingsFormValues, isValid: boolean) => {
      setFormValues(values);
      setIsFormValid(isValid);
    },
    [],
  );

  const handleSubmit = async () => {
    if (!isFormValid) return;
    let finalPath = formValues.path;
    const pathLooksUnc = /^\\\\|^\/\//.test(formValues.path.trim());

    if (runtimeInfo && pathLooksUnc) {
      const isWindows = runtimeInfo.platform === "windows";
      const isLinux = runtimeInfo.platform === "linux";
      const shouldConfigureWindows =
        isWindows &&
        (formValues.NetworkAuthEnabled ||
          Boolean(formValues.NetworkUsername || formValues.NetworkPassword));
      const shouldConfigureLinux = isLinux && runtimeInfo.supportsSambaMount;

      if (shouldConfigureWindows || shouldConfigureLinux) {
        const configured = await configureNetworkPath({
          path: formValues.path,
          username: formValues.NetworkUsername || undefined,
          password: formValues.NetworkPassword || undefined,
          mountPoint: shouldConfigureLinux
            ? formValues.NetworkMountPoint
            : undefined,
          persist: formValues.PersistNetworkCredentials,
          attemptConnect: true,
        });

        if (!configured.success) {
          addToast({
            title: "Network Path Error",
            description: configured.error ?? "Failed to configure network path",
            color: "danger",
          });
          return;
        }
        finalPath = configured.resolvedPath;
      }
    }

    await onAdd({
      name: formValues.name,
      path: finalPath,
      libraryType: formValues.libraryType,
      autoScan: formValues.autoScan,
      scanIntervalMinutes: formValues.scanIntervalMinutes,
      watchForChanges: formValues.watchForChanges,
      autoOrganize: formValues.autoOrganize,
      namingPattern: formValues.namingPattern ?? "",
    });

    // Reset form
    setFormValues(DEFAULT_LIBRARY_SETTINGS);
    setCloseSignal((s) => s + 1);
    onClose();
  };

  const handleClose = () => {
    setFormValues(DEFAULT_LIBRARY_SETTINGS);
    setCloseSignal((s) => s + 1);
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
            closeSignal={closeSignal}
            runtimePlatform={runtimeInfo?.platform}
            supportsUncCredentials={runtimeInfo?.supportsUncCredentials}
            supportsSambaMount={runtimeInfo?.supportsSambaMount}
            defaultLinuxMountBase={runtimeInfo?.defaultLinuxMountBase}
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
