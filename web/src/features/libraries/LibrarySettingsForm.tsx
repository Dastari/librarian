import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useNavigate } from "@tanstack/react-router";
import { IconFolder, IconTrash } from "@tabler/icons-react";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, ConfirmDialog, FieldGroup, FormNumberField, FormSelectField, FormSwitchField, FormTextField, Panel } from "@/components/ui";
import { FolderPickerDialog } from "@/features/files/FolderPickerDialog";
import {
  EntityLibraryDeleteDocument,
  EntityLibraryUpdateDocument,
  NamingPatternsForTypeDocument,
  QualityProfilesListDocument,
  type LibraryFieldsFragment,
} from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { libraryType } from "@/lib/library-types";

const schema = z.object({
  name: z.string().trim().min(1, "Give the library a name").max(80),
  path: z.string().trim().min(1, "Choose the folder that holds the media"),
  autoScan: z.boolean(),
  scanIntervalMinutes: z.number().int().min(5, "At least 5 minutes").max(10080, "At most a week"),
  watchForChanges: z.boolean(),
  autoOrganize: z.boolean(),
  namingPattern: z.string().nullable(),
  qualityProfileId: z.string().nullable(),
});
type Values = z.infer<typeof schema>;

export function LibrarySettingsForm({ library }: { library: LibraryFieldsFragment }) {
  const navigate = useNavigate();
  const meta = libraryType(library.libraryType);
  const [update, { loading: saving }] = useMutation(EntityLibraryUpdateDocument);
  const [remove, { loading: deleting }] = useMutation(EntityLibraryDeleteDocument);
  const patterns = useQuery(NamingPatternsForTypeDocument, { variables: { libraryType: library.libraryType.toLowerCase() } });
  const profiles = useQuery(QualityProfilesListDocument);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [pickingFolder, setPickingFolder] = useState(false);

  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: {
      name: library.name,
      path: library.path,
      autoScan: library.autoScan,
      scanIntervalMinutes: library.scanIntervalMinutes,
      watchForChanges: library.watchForChanges,
      autoOrganize: library.autoOrganize,
      namingPattern: library.namingPattern || null,
      qualityProfileId: library.qualityProfileId ?? null,
    },
  });

  useEffect(() => {
    form.reset({
      name: library.name,
      path: library.path,
      autoScan: library.autoScan,
      scanIntervalMinutes: library.scanIntervalMinutes,
      watchForChanges: library.watchForChanges,
      autoOrganize: library.autoOrganize,
      namingPattern: library.namingPattern || null,
      qualityProfileId: library.qualityProfileId ?? null,
    });
  }, [library, form]);

  const submit = form.handleSubmit(async (values) => {
    try {
      const { data } = await update({
        variables: {
          id: library.id,
          input: { ...values, namingPattern: values.namingPattern ?? "", qualityProfileId: values.qualityProfileId },
        },
      });
      assertSuccess(data?.updateLibrary, "Could not save the library");
      toast.success("Library saved");
      form.reset(values);
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  });

  const onDelete = async () => {
    try {
      const { data } = await remove({ variables: { id: library.id } });
      assertSuccess(data?.deleteLibrary, "Could not delete the library");
      toast.success("Library removed. Files on disk were left untouched.");
      await navigate({ to: "/libraries" });
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const patternOptions = (patterns.data?.namingPatterns.edges ?? []).map(({ node }) => ({ key: node.pattern, label: node.name, description: node.pattern }));
  const profileOptions = (profiles.data?.qualityProfiles.edges ?? []).map(({ node }) => ({ key: node.id, label: node.name, description: node.isDefault ? "Default" : undefined }));

  return (
    <form onSubmit={submit} noValidate className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
      <Panel title="Library" description={`${meta.label} library`}>
        <FieldGroup>
          <FormTextField control={form.control} name="name" label="Name" isRequired />
          <div className="flex items-end gap-2">
            <FormTextField control={form.control} name="path" label="Folder" mono isRequired className="flex-1" />
            <Button variant="secondary" onPress={() => setPickingFolder(true)} aria-label="Browse folders" isIconOnly className="mb-px">
              <IconFolder size={18} />
            </Button>
          </div>
          <FormSelectField control={form.control} name="qualityProfileId" label="Quality profile" options={profileOptions} placeholder="Use the default profile" />
        </FieldGroup>
      </Panel>

      <Panel title="Scanning">
        <FieldGroup>
          <FormSwitchField control={form.control} name="autoScan" label="Scan on a schedule" description="Look for new, changed and missing files automatically." />
          <FormNumberField control={form.control} name="scanIntervalMinutes" label="Scan every (minutes)" min={5} max={10080} step={5} />
          <FormSwitchField control={form.control} name="watchForChanges" label="Watch the folder" description="React to filesystem changes between scans." />
        </FieldGroup>
      </Panel>

      <Panel title="Organization">
        <FieldGroup>
          <FormSwitchField control={form.control} name="autoOrganize" label="Organize files" description="Rename and move matched files into the naming pattern below." />
          <FormSelectField control={form.control} name="namingPattern" label="Naming pattern" options={patternOptions} placeholder="Librarian default" />
        </FieldGroup>
      </Panel>

      <Panel title="Danger zone" tone="secondary">
        <p className="text-body-sm text-muted">Removing the library deletes its catalogue from Librarian. Files on disk are never touched.</p>
        <Button variant="danger-soft" className="mt-3" onPress={() => setConfirmDelete(true)}>
          <IconTrash size={16} /> Remove library
        </Button>
      </Panel>

      <div className="flex justify-end gap-2 lg:col-span-2">
        <Button variant="ghost" onPress={() => form.reset()} isDisabled={!form.formState.isDirty || saving}>
          Discard
        </Button>
        <Button type="submit" variant="primary" isPending={saving} isDisabled={!form.formState.isDirty}>
          Save changes
        </Button>
      </div>

      <ConfirmDialog
        isOpen={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={`Remove ${library.name}?`}
        description="The catalogue, playback history and match records for this library are deleted. Media files stay where they are."
        confirmLabel="Remove library"
        destructive
        isPending={deleting}
        onConfirm={onDelete}
      />
      <FolderPickerDialog isOpen={pickingFolder} onOpenChange={setPickingFolder} initialPath={form.getValues("path")} onPick={(path) => form.setValue("path", path, { shouldDirty: true })} />
    </form>
  );
}
