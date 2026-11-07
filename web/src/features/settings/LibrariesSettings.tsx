import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { zodResolver } from "@hookform/resolvers/zod";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconFolder, IconPlus } from "glass-ux/icons";
import { useCallback, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, Dialog, FieldGroup, FormSelectField, FormSwitchField, FormTextField } from "@/components/ui";
import { FolderPickerDialog } from "@/features/files/FolderPickerDialog";
import { libraryItemCount, type LibraryOverview } from "@/features/libraries/LibraryCard";
import { LibraryIdentity } from "@/features/libraries/LibraryIdentity";
import { LibrarySettingsDialog, type LibrarySettingsTab } from "@/features/libraries/LibrarySettingsDialog";
import { useLibraryActions } from "@/features/libraries/useLibraryActions";
import { EntityLibraryCreateDocument, LibrariesOverviewDocument } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { parseTimestamp, pluralize } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPE_OPTIONS, libraryType } from "@/lib/library-types";

import { TableBulkActions } from "@/components/ui/TableBulkActions";

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
    cell: ({ row }) => <LibraryIdentity library={row.original} items={libraryItemCount(row.original).count} />,
  }),
  column.accessor((row) => libraryType(row.libraryType).label, { id: "type", header: "Type", width: 8, priority: 2, sortable: true, cell: ({ row }) => <Badge size="sm" tone="neutral">{libraryType(row.original.libraryType).label}</Badge> }),
  column.accessor((row) => libraryItemCount(row).count, { id: "items", header: "Items", width: 8, priority: 2, align: "end", sortable: true, sortDescFirst: true, cell: ({ row }) => { const { count, noun } = libraryItemCount(row.original); return <TextCell muted={count === null}>{count === null ? "Files" : pluralize(count, noun)}</TextCell>; } }),
  column.display({ id: "schedule", header: "Scanning", width: 9, priority: 3, cell: ({ row }) => <TextCell muted>{row.original.autoScan ? `Every ${row.original.scanIntervalMinutes} min` : "Manual"}</TextCell> }),
  column.accessor((row) => row.lastScannedAt ?? "", { id: "scanned", header: "Last scan", width: 9, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.lastScannedAt)} fallback="Never" /> }),
  column.display({ id: "organize", header: "Organize", width: 7, priority: 4, cell: ({ row }) => <Badge size="sm" tone={row.original.autoOrganize ? "success" : "neutral"}>{row.original.autoOrganize ? "On" : "Off"}</Badge> }),
]);

export function LibrariesSettings() {
  const { user } = useSession();
  const list = useSettingsConnection(LibrariesOverviewDocument, {}, (data) => data.libraries);
  const { rows: libraries, loading, refetch } = list;
  const [creating, setCreating] = useState(false);
  const [pickingFolder, setPickingFolder] = useState(false);
  const [create, { loading: saving }] = useMutation(EntityLibraryCreateDocument);
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

  const [settings, setSettings] = useState<{ library: LibraryOverview; tab: LibrarySettingsTab } | null>(null);
  const openSettings = useCallback((library: LibraryOverview, tab: LibrarySettingsTab = "general") => setSettings({ library, tab }), []);
  const actions = useLibraryActions<LibraryOverview>({ settings: openSettings, onChanged: refetch });

  return (
    <div className="flex flex-col gap-6">
      <SettingsSection title="Libraries" actions={<Button variant="primary" size="sm" onPress={() => setCreating(true)}><IconPlus /> Add library</Button>}>
        <SettingsLoadError error={list.error} retry={list.refetch} hasRows={libraries.length > 0} />
        <LoadedDataTable<LibraryOverview> complete={!list.loading && !list.error} height="auto" selectable queryKey={user?.id}
          id={`librarian.${user?.id ?? "session"}.settings.libraries`}
          label="Libraries"
          columns={columns}
          rows={libraries}
          getRowId={(row) => row.id}
          getRowLabel={(row) => row.name}
          loading={loading && libraries.length === 0}
          actions={actions} renderBulkActions={(rows) => <TableBulkActions set={actions} rows={rows} noun={{ one: "library", other: "libraries" }} />}
          onRowOpen={(row) => openSettings(row)}
          rowOpen="click"
          noun={{ one: "library", other: "libraries" }}
          emptyMessage="No libraries yet."
          emptyAction={<Button variant="primary" onPress={() => setCreating(true)}><IconPlus /> Add library</Button>}
        />
      </SettingsSection>
      {settings ? <LibrarySettingsDialog library={libraries.find((row) => row.id === settings.library.id) ?? settings.library} open onOpenChange={(open) => { if (!open) setSettings(null); }} tab={settings.tab} onTabChange={(tab) => setSettings({ ...settings, tab })} items={libraryItemCount(settings.library).count} /> : null}

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
                <IconFolder size="lg" />
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
