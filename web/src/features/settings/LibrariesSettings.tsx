import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { zodResolver } from "@hookform/resolvers/zod";
import { Link } from "@tanstack/react-router";
import { IconFolder, IconPlus, IconSettings } from "@tabler/icons-react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, DataTable, type DataTableColumn, Dialog, EmptyState, FieldGroup, FormSelectField, FormSwitchField, FormTextField, Panel } from "@/components/ui";
import { FolderPickerDialog } from "@/features/files/FolderPickerDialog";
import { libraryItemCount, type LibraryOverview } from "@/features/libraries/LibraryCard";
import { EntityLibraryCreateDocument, LibrariesOverviewDocument } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { formatRelative, pluralize } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPE_OPTIONS, libraryType } from "@/lib/library-types";

const schema = z.object({
  name: z.string().trim().min(1, "Give the library a name").max(80),
  libraryType: z.string().min(1, "Choose a type"),
  path: z.string().trim().min(1, "Choose the folder"),
  autoScan: z.boolean(),
  autoOrganize: z.boolean(),
});
type Values = z.infer<typeof schema>;

export function LibrariesSettings() {
  const { user } = useSession();
  const { data, previousData, loading, refetch } = useQuery(LibrariesOverviewDocument);
  const libraries = (data ?? previousData)?.libraries.edges.map((edge) => edge.node) ?? [];
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

  const columns: Array<DataTableColumn<LibraryOverview>> = [
    {
      id: "name",
      header: "Library",
      cell: (row) => {
        const meta = libraryType(row.libraryType);
        return (
          <span className="flex items-center gap-3">
            <meta.icon size={18} className={meta.tint} />
            <span className="min-w-0">
              <span className="block truncate text-body-sm text-foreground">{row.name}</span>
              <span className="block truncate font-mono text-label-sm text-muted">{row.path}</span>
            </span>
          </span>
        );
      },
    },
    { id: "type", header: "Type", size: 120, cell: (row) => libraryType(row.libraryType).label },
    { id: "items", header: "Items", size: 120, align: "end", numeric: true, cell: (row) => { const { count, noun } = libraryItemCount(row); return pluralize(count, noun); } },
    { id: "scan", header: "Scanning", size: 160, hideBelow: "md", cell: (row) => <span className="text-muted">{row.autoScan ? `Every ${row.scanIntervalMinutes} min` : "Manual"}{row.lastScannedAt ? ` · ${formatRelative(row.lastScannedAt)}` : ""}</span> },
    { id: "organize", header: "Organize", size: 100, hideBelow: "lg", cell: (row) => <span className="text-muted">{row.autoOrganize ? "On" : "Off"}</span> },
    {
      id: "open",
      header: "",
      size: 60,
      align: "end",
      cell: (row) => (
        <Link to="/libraries/$libraryId/settings" params={{ libraryId: row.id }} aria-label="Library settings" className="nav-focus inline-grid size-8 place-items-center rounded-full text-muted hover:bg-surface-hover hover:text-foreground">
          <IconSettings size={16} />
        </Link>
      ),
    },
  ];

  return (
    <div className="flex flex-col gap-6">
      <Panel
        title="Libraries"
        flush
        actions={
          <Button variant="primary" size="sm" onPress={() => setCreating(true)}>
            <IconPlus size={16} /> Add library
          </Button>
        }
      >
        <DataTable<LibraryOverview>
          className="px-4 pb-4" frame={false}
          columns={columns}
          rows={libraries}
          getRowId={(row) => row.id}
          isLoading={loading && libraries.length === 0}
          noun="libraries"
          emptyState={<EmptyState compact icon={IconFolder} title="No libraries yet" action={<Button variant="primary" onPress={() => setCreating(true)}><IconPlus size={16} /> Add library</Button>} />}
        />
      </Panel>

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
              <FormTextField control={form.control} name="path" label="Folder" mono isRequired className="flex-1" placeholder="/data/media/Movies" />
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
