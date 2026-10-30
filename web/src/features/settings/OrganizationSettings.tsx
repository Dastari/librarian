import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { IconFolders, IconPlus, IconTrash } from "@tabler/icons-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { z } from "zod";

import { Button, ConfirmDialog, DataTable, type DataTableColumn, type DataTableRowAction, Dialog, EmptyState, FieldGroup, Panel, StatusChip } from "@/components/ui";
import { EntityNamingPatternCreateDocument, EntityNamingPatternDeleteDocument, EntityNamingPatternListDocument, EntityNamingPatternUpdateDocument, type NamingPatternFieldsFragment } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPE_OPTIONS, libraryType } from "@/lib/library-types";

import { asBool, useAppSettings } from "./useAppSettings";

import { SettingsLoadError } from "./SettingsLoadError";
import { useSettingsConnection } from "./useSettingsConnection";

const globalSchema = z.object({ autoOrganize: z.boolean(), deleteEmptyFolders: z.boolean(), copyMode: z.string() });
type GlobalValues = z.infer<typeof globalSchema>;

const patternSchema = z.object({ name: z.string().trim().min(1, "Name the pattern"), libraryType: z.string().min(1), pattern: z.string().trim().min(1, "Enter the pattern"), description: z.string().trim(), isDefault: z.boolean() });
type PatternValues = z.infer<typeof patternSchema>;

export function OrganizationSettings() {
  const { user } = useSession();
  const organize = useAppSettings("organize");
  const globalDefaults = useRef<GlobalValues>({ autoOrganize: false, deleteEmptyFolders: true, copyMode: "hardlink" });
  const globalForm = useAppForm({ ...formBehaviour, validators: { onDynamic: globalSchema }, defaultValues: globalDefaults.current,
    onSubmit: async ({ value }) => {
      const values = globalSchema.parse(value);
      await organize.save({ "organize.auto_organize": values.autoOrganize, "organize.delete_empty_folders": values.deleteEmptyFolders, "organize.copy_mode": values.copyMode });
      globalDefaults.current = values;
      toast.success("Settings saved");
    },
  });
  useEffect(() => {
    const snapshot: GlobalValues = { autoOrganize: asBool(organize.values.get("organize.auto_organize"), false), deleteEmptyFolders: asBool(organize.values.get("organize.delete_empty_folders"), true), copyMode: organize.values.get("organize.copy_mode") || "hardlink" };
    const current = globalForm.state.values;
    const baseline = globalDefaults.current;
    const edited = (Object.keys(snapshot) as Array<keyof GlobalValues>).filter((key) => !Object.is(current[key], baseline[key]));
    globalDefaults.current = snapshot;
    globalForm.reset(snapshot);
    for (const key of edited) globalForm.setFieldValue(key, current[key]);
  }, [organize.values, globalForm]);

  const patterns = useSettingsConnection(EntityNamingPatternListDocument, { orderBy: [{ libraryType: "ASC" }, { name: "ASC" }] }, (data) => data.namingPatterns);
  const rows = patterns.rows;
  const [editing, setEditing] = useState<NamingPatternFieldsFragment | null | "new">(null);
  const [removing, setRemoving] = useState<NamingPatternFieldsFragment | null>(null);
  const [create] = useMutation(EntityNamingPatternCreateDocument);
  const [update] = useMutation(EntityNamingPatternUpdateDocument);
  const [remove, { loading: deleting }] = useMutation(EntityNamingPatternDeleteDocument);
  const patternDefaults = useMemo<PatternValues>(() => {
    const source = editing === "new" ? null : editing;
    return { name: source?.name ?? "", libraryType: source?.libraryType ?? "movies", pattern: source?.pattern ?? "", description: source?.description ?? "", isDefault: source?.isDefault ?? false };
  }, [editing]);
  const patternForm = useAppForm({ ...formBehaviour, validators: { onDynamic: patternSchema }, defaultValues: patternDefaults,
    onSubmit: async ({ value }) => {
      if (!user) throw new Error("Sign in to save a naming pattern");
      const values = patternSchema.parse(value);
      if (editing && editing !== "new") assertSuccess((await update({ variables: { id: editing.id, input: { ...values, description: values.description || null } } })).data?.updateNamingPattern, "Could not save");
      else assertSuccess((await create({ variables: { input: { ...values, description: values.description || null, userId: user.id, isSystem: false } } })).data?.createNamingPattern, "Could not create");
      toast.success("Pattern saved");
      setEditing(null);
      void patterns.refetch();
    },
  });
  useEffect(() => {
    if (editing !== null) patternForm.reset(patternDefaults);
  }, [editing, patternDefaults, patternForm]);

  const columns = useMemo<Array<DataTableColumn<NamingPatternFieldsFragment>>>(() => [
    { id: "name", header: "Pattern", cell: (row) => (<span className="min-w-0"><span className="block truncate text-body-sm text-foreground">{row.name}</span><span className="block truncate font-mono text-label-sm text-muted">{row.pattern}</span></span>) },
    { id: "type", header: "Library type", size: 130, cell: (row) => libraryType(row.libraryType).label },
    { id: "flags", header: "", size: 160, cell: (row) => (<span className="flex gap-1">{row.isDefault ? <StatusChip status={{ label: "Default", tone: "accent", dot: "bg-info" }} /> : null}{row.isSystem ? <StatusChip status={{ label: "Built-in", tone: "default", dot: "bg-muted" }} /> : null}</span>) },
  ], []);
  const actions: Array<DataTableRowAction<NamingPatternFieldsFragment>> = [
    { key: "default", label: "Make default for type", hidden: (row) => row.isDefault, onAction: async (row) => { try { for (const other of rows.filter((item) => item.libraryType === row.libraryType && item.isDefault)) await update({ variables: { id: other.id, input: { isDefault: false } } }); await update({ variables: { id: row.id, input: { isDefault: true } } }); void patterns.refetch(); } catch (error) { toast.danger(errorMessage(error)); } } },
    { key: "delete", label: "Delete", icon: <IconTrash size={16} />, destructive: true, hidden: (row) => row.isSystem, onAction: (row) => setRemoving(row) },
  ];

  return (
    <div className="flex flex-col gap-6">
      <Form form={globalForm} className="flex min-w-0 flex-col gap-6" onError={(error) => toast.danger(errorMessage(error, "Could not save settings"))}>
        <Panel title="File handling">
          <FieldGroup>
            <globalForm.AppField name="autoOrganize">{(field) => <field.SwitchRow label="Organize imported files by default" description="Per-library settings override this." />}</globalForm.AppField>
            <globalForm.AppField name="deleteEmptyFolders">{(field) => <field.SwitchRow label="Remove empty folders after moving files" description="Folders that belong to the naming structure are always kept." />}</globalForm.AppField>
            <globalForm.AppField name="copyMode">{(field) => <field.SelectField label="Import method" options={[{ value: "hardlink", label: "Hard link, copy if not possible", detail: "Keeps seeding without using extra space" }, { value: "copy", label: "Always copy" }, { value: "move", label: "Move", detail: "Source files are removed once the import succeeds" }]} />}</globalForm.AppField>
          </FieldGroup>
        </Panel>
        <FormActions>
          <globalForm.Subscribe selector={(state) => state.isSubmitting}>{(pending) => <globalForm.CancelButton disabled={pending}>Discard</globalForm.CancelButton>}</globalForm.Subscribe>
          <globalForm.SubmitButton>Save</globalForm.SubmitButton>
        </FormActions>
      </Form>

      <Panel title="Naming patterns" description="Tokens such as {title}, {year}, {season:00}, {episode:00}, {quality} are replaced when files are organized." flush actions={<Button variant="primary" size="sm" onPress={() => setEditing("new")}><IconPlus size={16} /> New pattern</Button>}>
        <SettingsLoadError error={patterns.error} retry={patterns.refetch} hasRows={rows.length > 0} />
        <DataTable<NamingPatternFieldsFragment> className="px-4 pb-4" frame={false} columns={columns} rows={rows} getRowId={(row) => row.id} isLoading={patterns.loading && rows.length === 0} rowActions={actions} onRowClick={(row) => (row.isSystem ? undefined : setEditing(row))} noun="patterns" density="compact" emptyState={<EmptyState compact icon={IconFolders} title="No naming patterns" />} />
      </Panel>

      <Dialog isOpen={editing !== null} onOpenChange={(open) => !open && setEditing(null)} title={editing === "new" || !editing ? "New naming pattern" : `Edit ${editing.name}`} size="md">
        <Form form={patternForm} onError={(error) => toast.danger(errorMessage(error))}>
          <FieldGroup>
            <patternForm.AppField name="name">{(field) => <field.TextField label="Name" required />}</patternForm.AppField>
            <patternForm.AppField name="libraryType">{(field) => <field.SelectField label="Library type" options={LIBRARY_TYPE_OPTIONS.map((option) => ({ value: option.type, label: option.label }))} />}</patternForm.AppField>
            <patternForm.AppField name="pattern">{(field) => <field.TextField label="Pattern" className="font-mono" required placeholder="{title} ({year})/{title} ({year}) - {quality}" />}</patternForm.AppField>
            <patternForm.AppField name="description">{(field) => <field.TextField label="Description" />}</patternForm.AppField>
            <patternForm.AppField name="isDefault">{(field) => <field.SwitchRow label="Default for this library type" />}</patternForm.AppField>
          </FieldGroup>
          <FormActions>
            <patternForm.Subscribe selector={(state) => state.isSubmitting}>{(pending) => <patternForm.CancelButton disabled={pending} onCancel={() => setEditing(null)}>Cancel</patternForm.CancelButton>}</patternForm.Subscribe>
            <patternForm.SubmitButton>Save</patternForm.SubmitButton>
          </FormActions>
        </Form>
      </Dialog>
      <ConfirmDialog isOpen={Boolean(removing)} onOpenChange={(open) => !open && setRemoving(null)} title={`Delete ${removing?.name}?`} confirmLabel="Delete" destructive isPending={deleting} onConfirm={async () => { if (!removing) return; try { assertSuccess((await remove({ variables: { id: removing.id } })).data?.deleteNamingPattern, "Could not delete"); setRemoving(null); void patterns.refetch(); } catch (error) { toast.danger(errorMessage(error)); } }} />
    </div>
  );
}
