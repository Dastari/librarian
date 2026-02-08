import { useState, useEffect, useCallback, useMemo } from "react";
import { addToast } from "@heroui/toast";
import {
  LibrarySettingsForm,
  type LibrarySettingsFormValues,
} from "./LibrarySettingsForm";
import { SettingsHeader } from "../shared";
import type {
  UpdateLibraryInput,
  Library as LibraryEntity,
} from "../../lib/graphql/generated/graphql";

type LibrarySettingsData = Pick<
  LibraryEntity,
  | "Name"
  | "Path"
  | "LibraryType"
  | "AutoScan"
  | "ScanIntervalMinutes"
  | "WatchForChanges"
  | "AutoOrganize"
  | "NamingPattern"
>;

interface LibrarySettingsTabProps {
  library: LibrarySettingsData;
  onSave: (input: UpdateLibraryInput) => Promise<void>;
  isLoading: boolean;
}

export function LibrarySettingsTab({
  library,
  onSave,
  isLoading,
}: LibrarySettingsTabProps) {
  // Convert Library entity to form values
  const libraryToFormValues = useCallback(
    (lib: LibrarySettingsData): LibrarySettingsFormValues => ({
      Name: lib.Name,
      Path: lib.Path,
      LibraryType: lib.LibraryType as LibrarySettingsFormValues["LibraryType"],
      AutoScan: lib.AutoScan,
      ScanIntervalMinutes: lib.ScanIntervalMinutes,
      WatchForChanges: lib.WatchForChanges,
      AutoOrganize: lib.AutoOrganize,
      NamingPattern: lib.NamingPattern || null,
      NetworkAuthEnabled: false,
      NetworkUsername: "",
      NetworkPassword: "",
      NetworkMountPoint: "/mnt",
      PersistNetworkCredentials: true,
    }),
    [],
  );

  const [formValues, setFormValues] = useState<LibrarySettingsFormValues>(() =>
    libraryToFormValues(library),
  );
  const [isFormValid, setIsFormValid] = useState(true);
  const [hasChanges, setHasChanges] = useState(false);

  // Reset form when library changes
  useEffect(() => {
    setFormValues(libraryToFormValues(library));
    setHasChanges(false);
  }, [library, libraryToFormValues]);

  // Track changes by comparing with original values
  const originalValues = useMemo(
    () => libraryToFormValues(library),
    [library, libraryToFormValues],
  );

  useEffect(() => {
    const changed =
      formValues.Name !== originalValues.Name ||
      formValues.Path !== originalValues.Path ||
      formValues.AutoScan !== originalValues.AutoScan ||
      formValues.ScanIntervalMinutes !== originalValues.ScanIntervalMinutes ||
      formValues.WatchForChanges !== originalValues.WatchForChanges ||
      formValues.AutoOrganize !== originalValues.AutoOrganize ||
      formValues.NamingPattern !== originalValues.NamingPattern;

    setHasChanges(changed);
  }, [formValues, originalValues]);

  const handleChange = useCallback(
    (values: LibrarySettingsFormValues, isValid: boolean) => {
      setFormValues(values);
      setIsFormValid(isValid);
    },
    [],
  );

  const handleSubmit = async () => {
    if (!isFormValid) {
      addToast({
        title: "Validation Error",
        description: "Please fix the form errors before saving",
        color: "danger",
      });
      return;
    }

    await onSave({
      Name: formValues.Name,
      Path: formValues.Path,
      LibraryType: formValues.LibraryType,
      AutoScan: formValues.AutoScan,
      ScanIntervalMinutes: formValues.ScanIntervalMinutes,
      WatchForChanges: formValues.WatchForChanges,
      AutoOrganize: formValues.AutoOrganize,
      NamingPattern: formValues.NamingPattern,
    });
  };

  const handleReset = useCallback(() => {
    setFormValues(libraryToFormValues(library));
  }, [library, libraryToFormValues]);

  return (
    <div
      className="grow overflow-hidden overflow-y-auto pb-8 px-4"
      style={{ scrollbarGutter: "stable" }}
    >
      <SettingsHeader
        title="Library Settings"
        subtitle="Configure how this library behaves"
        onSave={handleSubmit}
        onReset={handleReset}
        isSaveDisabled={!hasChanges || !isFormValid}
        isResetDisabled={!hasChanges}
        isSaving={isLoading}
        hasChanges={hasChanges}
      />

      <LibrarySettingsForm
        initialValues={formValues}
        onChange={handleChange}
        mode="edit"
        useCards={true}
      />
    </div>
  );
}
