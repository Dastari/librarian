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

// `qualityProfileId` is not yet in the generated `Library`/`UpdateLibraryInput`
// types: codegen introspects the *running* backend, and this environment's
// server process could not be restarted to pick up the new `QualityProfile`
// schema (see docs/tier1-features-plan.md §2 "Implemented" note). Extend the
// generated types by hand for now; remove this once codegen is re-run against
// the updated schema.
type LibrarySettingsData = Pick<
  LibraryEntity,
  | "name"
  | "path"
  | "libraryType"
  | "autoScan"
  | "scanIntervalMinutes"
  | "watchForChanges"
  | "autoOrganize"
  | "namingPattern"
> & { qualityProfileId?: string | null };

type UpdateLibraryInputWithQualityProfile = UpdateLibraryInput & {
  qualityProfileId?: string | null;
};

interface LibrarySettingsTabProps {
  library: LibrarySettingsData;
  onSave: (input: UpdateLibraryInputWithQualityProfile) => Promise<void>;
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
      name: lib.name,
      path: lib.path,
      libraryType: lib.libraryType as LibrarySettingsFormValues["libraryType"],
      autoScan: lib.autoScan,
      scanIntervalMinutes: lib.scanIntervalMinutes,
      watchForChanges: lib.watchForChanges,
      autoOrganize: lib.autoOrganize,
      namingPattern: lib.namingPattern || null,
      qualityProfileId: lib.qualityProfileId ?? null,
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
      formValues.name !== originalValues.name ||
      formValues.path !== originalValues.path ||
      formValues.autoScan !== originalValues.autoScan ||
      formValues.scanIntervalMinutes !== originalValues.scanIntervalMinutes ||
      formValues.watchForChanges !== originalValues.watchForChanges ||
      formValues.autoOrganize !== originalValues.autoOrganize ||
      formValues.namingPattern !== originalValues.namingPattern ||
      formValues.qualityProfileId !== originalValues.qualityProfileId;

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
      name: formValues.name,
      path: formValues.path,
      libraryType: formValues.libraryType,
      autoScan: formValues.autoScan,
      scanIntervalMinutes: formValues.scanIntervalMinutes,
      watchForChanges: formValues.watchForChanges,
      autoOrganize: formValues.autoOrganize,
      namingPattern: formValues.namingPattern,
      qualityProfileId: formValues.qualityProfileId,
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
