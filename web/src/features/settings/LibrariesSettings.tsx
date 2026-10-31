import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { zodResolver } from "@hookform/resolvers/zod";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconExternalLink, IconPlus, IconRefresh, IconSettings } from "glass-ux/icons";
import { useNavigate } from "@tanstack/react-router";
import { IconFolder } from "@tabler/icons-react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, Dialog, FieldGroup, FormSelectField, FormSwitchField, FormTextField } from "@/components/ui";
import { FolderPickerDialog } from "@/features/files/FolderPickerDialog";
import { libraryItemCount, type LibraryOverview } from "@/features/libraries/LibraryCard";
import { EntityLibraryCreateDocument, LibrariesOverviewDocument, ScanLibraryDocument } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { parseTimestamp, pluralize } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPE_OPTIONS, libraryType } from "@/lib/library-types";

import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsSection } from "./SettingsSection";
import { useSettingsConnection } from "./useSettingsConnection";

const schema = z.object({
  name: z.string().trim().min(1, "Give the library a name").max(80),
  libraryType: z.string().min(1, "Choose a type"),
  path: z.string().trim().min(1, "Choose the folder"),
  autoScan: z.boolean(),
  autoOrganize: z.boolean(),
});
type Values = z.infer<typeof schema>;

const column = createDataTableColumns<LibraryOverview>();
const columns = column.columns([
  column.accessor("name", {
    header: "Library", width: 26, priority: 1, cardRole: "title", sortable: true,
    cell: ({ row }) => { const meta = libraryType(row.original.libraryType); return <IdentityCell icon={<meta.icon className={meta.tint} />} title={row.original.name} subtitle={row.original.path} status={row.original.scanning ? "installing" : "online"} statusLabel={row.original.scanning ? "Scanning" : "Idle"} />; },
  }),
  column.accessor((row) => libraryType(row.libraryType).label, { id: "type", header: "Type", width: 8, priority: 2, sortable: true, cell: ({ row }) => <Badge size="sm" tone="neutral">{libraryType(row.original.libraryType).label}</Badge> }),
  column.accessor((row) => libraryItemCount(row).count, { id: "items", header: "Items", width: 8, priority: 2, align: "end", sortable: true, sortDescFirst: true, cell: ({ row }) => { const { count, noun } = libraryItemCount(row.original); return <TextCell>{pluralize(count, noun)}</TextCell>; } }),
  column.display({ id: "schedule", header: "Scanning", width: 9, priority: 3, cell: ({ row }) => <TextCell muted>{row.original.autoScan ? `Every ${row.original.scanIntervalMinutes} min` : "Manual"}</TextCell> }),
  column.accessor((row) => row.lastScannedAt ?? "", { id: "scanned", header: "Last scan", width: 9, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.lastScannedAt)} fallback="Never" /> }),
  column.display({ id: "organize", header: "Organize", width: 7, priority: 4, cell: ({ row }) => <Badge size="sm" tone={row.original.autoOrganize ? "success" : "neutral"}>{row.original.autoOrganize ? "On" : "Off"}</Badge> }),
]);

export function LibrariesSettings() {
  const { user } = useSession();
  const navigate = useNavigate();
  const list = useSettingsConnection(LibrariesOverviewDocument, {}, (data) => data.libraries);
  const { rows: libraries, loading, refetch } = list;
  const [creating, setCreating] = useState(false);
  const [pickingFolder, setPickingFolder] = useState(false);
  const [create, { loading: saving }] = useMutation(EntityLibraryCreateDocument);
  const [scan] = useMutation(ScanLibraryDocument);
  const form = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { name: "", libraryType: "movies", path: "", autoScan: true, autoOrganize: false } });

  const submit = form.handleSubmit(async (values) => {
    if (!user) return;
    try {
      const { data: result } = await create({
        variables: { input: { userId: user.id, name: values.name, path: values.path, libraryType: values.libraryType, autoScan: values.autoScan, autoOrganize: values.autoOrganize, namingPattern: "", scanIntervalMinutes: 60, watchForChanges: false, scanning: false } },
      });
      assertSuccess(result?.createLibrary, "Could not create the library");
      toast.success(`${values.name} created. Run a scan to import existing files.`);
      form.reset();
      setCreating(false);
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  });

  const actions = defineActions<LibraryOverview>("library", [
    { id: "open", label: "Open library", icon: IconExternalLink, group: "open", priority: 1, visible: (rows) => rows.length === 1, run: (rows, context) => context.navigate(`/libraries/${rows[0]!.id}`) },
    { id: "settings", label: "Library settings", icon: IconSettings, group: "open", priority: 2, visible: (rows) => rows.length === 1, run: (rows, context) => context.navigate(`/libraries/${rows[0]!.id}/settings`) },
    {
      id: "scan", label: (rows) => (rows.length === 1 ? "Scan now" : `Scan ${rows.length} libraries`), icon: IconRefresh, group: "manage", priority: 1, bulk: true,
      disabledReason: (rows) => (rows.every((row) => row.scanning) ? "Already scanning" : false),
      run: async (rows) => {
        for (const row of rows.filter((item) => !item.scanning)) {
          const result = (await scan({ variables: { id: row.id } })).data?.scanLibrary;
          if (!result?.success) throw new Error(result?.message ?? `${row.name}: the scan could not start`);
        }
        void refetch();
      },
      successMessage: (rows) => (rows.length === 1 ? "Scan started" : `Started ${rows.length} scans`),
    },
  ]);

  return (
    <div className="flex flex-col gap-6">
      <SettingsSection title="Libraries" actions={<Button variant="primary" size="sm" onPress={() => setCreating(true)}><IconPlus /> Add library</Button>}>
        <SettingsLoadError error={list.error} retry={list.refetch} hasRows={libraries.length > 0} />
        <DataTable<LibraryOverview>
          id={`librarian.${user?.id ?? "session"}.settings.libraries`}
          label="Libraries"
          columns={columns}
          rows={libraries}
          getRowId={(row) => row.id}
          getRowLabel={(row) => row.name}
          loading={loading && libraries.length === 0}
          actions={actions}
          selectable
          onRowOpen={(row) => void navigate({ to: "/libraries/$libraryId/settings", params: { libraryId: row.id } })}
          rowOpen="click"
          noun={{ one: "library", other: "libraries" }}
          emptyMessage="No libraries yet."
          emptyAction={<Button variant="primary" onPress={() => setCreating(true)}><IconPlus /> Add library</Button>}
        />
      </SettingsSection>

      <Dialog
        isOpen={creating}
        onOpenChange={setCreating}
        title="Add library"
        size="md"
        footer={
          <>
            <Button variant="ghost" onPress={() => setCreating(false)}>
              Cancel
            </Button>
            <Button variant="primary" onPress={() => void submit()} isPending={saving}>
              Create
            </Button>
          </>
        }
      >
        <form onSubmit={submit} noValidate>
          <FieldGroup>
            <FormTextField control={form.control} name="name" label="Name" placeholder="Movies" autoFocus isRequired />
            <FormSelectField control={form.control} name="libraryType" label="Type" options={LIBRARY_TYPE_OPTIONS.map((option) => ({ key: option.type, label: option.label }))} isRequired />
            <div className="flex items-end gap-2">
              <FormTextField control={form.control} name="path" label="Folder" mono isRequired className="min-w-0 flex-1" placeholder="/data/media/Movies" />
              <Button variant="secondary" isIconOnly aria-label="Browse folders" onPress={() => setPickingFolder(true)} className="mb-px">
                <IconFolder size={18} />
              </Button>
            </div>
            <FormSwitchField control={form.control} name="autoScan" label="Scan on a schedule" />
            <FormSwitchField control={form.control} name="autoOrganize" label="Organize files into the naming pattern" />
          </FieldGroup>
        </form>
      </Dialog>
      <FolderPickerDialog isOpen={pickingFolder} onOpenChange={setPickingFolder} initialPath={form.getValues("path")} onPick={(path) => form.setValue("path", path, { shouldDirty: true, shouldValidate: true })} />
    </div>
  );
}
