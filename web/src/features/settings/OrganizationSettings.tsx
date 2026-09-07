import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { zodResolver } from "@hookform/resolvers/zod";
import { IconFolders, IconPlus, IconTrash } from "@tabler/icons-react";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, ConfirmDialog, DataTable, type DataTableColumn, type DataTableRowAction, Dialog, EmptyState, FieldGroup, FormSelectField, FormSwitchField, FormTextField, Panel, StatusChip } from "@/components/ui";
import { EntityNamingPatternCreateDocument, EntityNamingPatternDeleteDocument, EntityNamingPatternListDocument, EntityNamingPatternUpdateDocument, type NamingPatternFieldsFragment } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPE_OPTIONS, libraryType } from "@/lib/library-types";

import { SettingsForm } from "./SettingsForm";
import { asBool, useAppSettings } from "./useAppSettings";

const globalSchema = z.object({ autoOrganize: z.boolean(), deleteEmptyFolders: z.boolean(), copyMode: z.string() });
type GlobalValues = z.infer<typeof globalSchema>;

const patternSchema = z.object({ name: z.string().trim().min(1, "Name the pattern"), libraryType: z.string().min(1), pattern: z.string().trim().min(1, "Enter the pattern"), description: z.string().trim(), isDefault: z.boolean() });
type PatternValues = z.infer<typeof patternSchema>;

export function OrganizationSettings() {
  const { user } = useSession();
  const organize = useAppSettings("organize");
  const globalForm = useForm<GlobalValues>({ resolver: zodResolver(globalSchema), defaultValues: { autoOrganize: false, deleteEmptyFolders: true, copyMode: "hardlink" } });
  useEffect(() => {
    globalForm.reset({ autoOrganize: asBool(organize.values.get("organize.auto_organize"), false), deleteEmptyFolders: asBool(organize.values.get("organize.delete_empty_folders"), true), copyMode: organize.values.get("organize.copy_mode") || "hardlink" });
  }, [organize.values, globalForm]);

  const patterns = useQuery(EntityNamingPatternListDocument, { variables: { orderBy: [{ libraryType: "ASC" }, { name: "ASC" }], page: { limit: 100, offset: 0 } } });
  const rows = (patterns.data ?? patterns.previousData)?.namingPatterns.edges.map((edge) => edge.node) ?? [];
  const [editing, setEditing] = useState<NamingPatternFieldsFragment | null | "new">(null);
  const [removing, setRemoving] = useState<NamingPatternFieldsFragment | null>(null);
  const [create, { loading: creating }] = useMutation(EntityNamingPatternCreateDocument);
  const [update, { loading: updating }] = useMutation(EntityNamingPatternUpdateDocument);
  const [remove, { loading: deleting }] = useMutation(EntityNamingPatternDeleteDocument);
  const patternForm = useForm<PatternValues>({ resolver: zodResolver(patternSchema), defaultValues: { name: "", libraryType: "movies", pattern: "", description: "", isDefault: false } });
  useEffect(() => {
    if (editing !== null) {
      const source = editing === "new" ? null : editing;
      patternForm.reset({ name: source?.name ?? "", libraryType: source?.libraryType ?? "movies", pattern: source?.pattern ?? "", description: source?.description ?? "", isDefault: source?.isDefault ?? false });
    }
  }, [editing, patternForm]);

  const submitPattern = patternForm.handleSubmit(async (values) => {
    if (!user) return;
    try {
      if (editing && editing !== "new") assertSuccess((await update({ variables: { id: editing.id, input: { ...values, description: values.description || null } } })).data?.updateNamingPattern, "Could not save");
      else assertSuccess((await create({ variables: { input: { ...values, description: values.description || null, userId: user.id, isSystem: false } } })).data?.createNamingPattern, "Could not create");
      toast.success("Pattern saved");
      setEditing(null);
      void patterns.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  });

  const columns: Array<DataTableColumn<NamingPatternFieldsFragment>> = [
    { id: "name", header: "Pattern", cell: (row) => (<span className="min-w-0"><span className="block truncate text-body-sm text-foreground">{row.name}</span><span className="block truncate font-mono text-label-sm text-muted">{row.pattern}</span></span>) },
    { id: "type", header: "Library type", size: 130, cell: (row) => libraryType(row.libraryType).label },
    { id: "flags", header: "", size: 160, cell: (row) => (<span className="flex gap-1">{row.isDefault ? <StatusChip status={{ label: "Default", tone: "accent", dot: "bg-info" }} /> : null}{row.isSystem ? <StatusChip status={{ label: "Built-in", tone: "default", dot: "bg-muted" }} /> : null}</span>) },
  ];
  const actions: Array<DataTableRowAction<NamingPatternFieldsFragment>> = [
    { key: "default", label: "Make default for type", hidden: (row) => row.isDefault, onAction: async (row) => { try { for (const other of rows.filter((item) => item.libraryType === row.libraryType && item.isDefault)) await update({ variables: { id: other.id, input: { isDefault: false } } }); await update({ variables: { id: row.id, input: { isDefault: true } } }); void patterns.refetch(); } catch (error) { toast.danger(errorMessage(error)); } } },
    { key: "delete", label: "Delete", icon: <IconTrash size={16} />, destructive: true, hidden: (row) => row.isSystem, onAction: (row) => setRemoving(row) },
  ];

  return (
    <div className="flex flex-col gap-6">
      <SettingsForm form={globalForm} onSave={async (values) => organize.save({ "organize.auto_organize": values.autoOrganize, "organize.delete_empty_folders": values.deleteEmptyFolders, "organize.copy_mode": values.copyMode })}>
        <Panel title="File handling">
          <FieldGroup>
            <FormSwitchField control={globalForm.control} name="autoOrganize" label="Organize imported files by default" description="Per-library settings override this." />
            <FormSwitchField control={globalForm.control} name="deleteEmptyFolders" label="Remove empty folders after moving files" description="Folders that belong to the naming structure are always kept." />
            <FormSelectField control={globalForm.control} name="copyMode" label="Import method" options={[{ key: "hardlink", label: "Hard link, copy if not possible", description: "Keeps seeding without using extra space" }, { key: "copy", label: "Always copy" }, { key: "move", label: "Move", description: "Source files are removed once the import succeeds" }]} />
          </FieldGroup>
        </Panel>
      </SettingsForm>

      <Panel title="Naming patterns" description="Tokens such as {title}, {year}, {season:00}, {episode:00}, {quality} are replaced when files are organized." flush actions={<Button variant="primary" size="sm" onPress={() => setEditing("new")}><IconPlus size={16} /> New pattern</Button>}>
        <DataTable<NamingPatternFieldsFragment> className="px-4 pb-4" frame={false} columns={columns} rows={rows} getRowId={(row) => row.id} isLoading={patterns.loading && rows.length === 0} rowActions={actions} onRowClick={(row) => (row.isSystem ? undefined : setEditing(row))} noun="patterns" density="compact" emptyState={<EmptyState compact icon={IconFolders} title="No naming patterns" />} />
      </Panel>

      <Dialog isOpen={editing !== null} onOpenChange={(open) => !open && setEditing(null)} title={editing === "new" || !editing ? "New naming pattern" : `Edit ${editing.name}`} size="md" footer={<><Button variant="ghost" onPress={() => setEditing(null)}>Cancel</Button><Button variant="primary" onPress={() => void submitPattern()} isPending={creating || updating}>Save</Button></>}>
        <form onSubmit={submitPattern} noValidate>
          <FieldGroup>
            <FormTextField control={patternForm.control} name="name" label="Name" isRequired autoFocus />
            <FormSelectField control={patternForm.control} name="libraryType" label="Library type" options={LIBRARY_TYPE_OPTIONS.map((option) => ({ key: option.type, label: option.label }))} />
            <FormTextField control={patternForm.control} name="pattern" label="Pattern" mono isRequired placeholder="{title} ({year})/{title} ({year}) - {quality}" />
            <FormTextField control={patternForm.control} name="description" label="Description" />
            <FormSwitchField control={patternForm.control} name="isDefault" label="Default for this library type" />
          </FieldGroup>
        </form>
      </Dialog>
      <ConfirmDialog isOpen={Boolean(removing)} onOpenChange={(open) => !open && setRemoving(null)} title={`Delete ${removing?.name}?`} confirmLabel="Delete" destructive isPending={deleting} onConfirm={async () => { if (!removing) return; try { assertSuccess((await remove({ variables: { id: removing.id } })).data?.deleteNamingPattern, "Could not delete"); setRemoving(null); void patterns.refetch(); } catch (error) { toast.danger(errorMessage(error)); } }} />
    </div>
  );
}
