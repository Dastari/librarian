import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconPlus } from "glass-ux/icons";
import { useCallback, useState } from "react";

import { Button } from "@/components/ui";
import { AddLibraryDialog } from "@/features/libraries/AddLibraryDialog";
import { libraryItemCount, type LibraryOverview } from "@/features/libraries/LibraryCard";
import { LibraryIdentity } from "@/features/libraries/LibraryIdentity";
import { libraryWatch } from "@/features/libraries/libraryWatch";
import { LibrarySettingsDialog, type LibrarySettingsTab } from "@/features/libraries/LibrarySettingsDialog";
import { useLibraryActions } from "@/features/libraries/useLibraryActions";
import { LibrariesOverviewDocument } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { parseTimestamp, pluralize } from "@/lib/format";
import { libraryType } from "@/lib/library-types";

import { TableBulkActions } from "@/components/ui/TableBulkActions";

import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsPage, SettingsSection } from "./SettingsSection";
import { useSettingsConnection } from "./useSettingsConnection";

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
  column.display({ id: "watch", header: "Watch", width: 9, priority: 4, cell: ({ row }) => { const watch = libraryWatch(row.original); return <Badge size="sm" marker={watch.id === "off" ? undefined : "dot"} tone={watch.tone}>{watch.id === "off" ? "Off" : watch.id === "watching" ? "Watching" : watch.id === "unavailable" ? "Not reachable" : watch.id === "failed" ? "Failed" : "Starting"}</Badge>; } }),
  column.display({ id: "organize", header: "Organize", width: 7, priority: 4, cell: ({ row }) => <Badge size="sm" tone={row.original.autoOrganize ? "success" : "neutral"}>{row.original.autoOrganize ? "On" : "Off"}</Badge> }),
]);

export function LibrariesSettings() {
  const { user } = useSession();
  const list = useSettingsConnection(LibrariesOverviewDocument, {}, (data) => data.libraries);
  const { rows: libraries, loading, refetch } = list;
  const [creating, setCreating] = useState(false);
  const [settings, setSettings] = useState<{ library: LibraryOverview; tab: LibrarySettingsTab } | null>(null);
  const openSettings = useCallback((library: LibraryOverview, tab: LibrarySettingsTab = "general") => setSettings({ library, tab }), []);
  const actions = useLibraryActions<LibraryOverview>({ settings: openSettings, onChanged: refetch });

  return (
    <SettingsPage>
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

      <AddLibraryDialog open={creating} onOpenChange={setCreating} onCreated={() => void refetch()} />
    </SettingsPage>
  );
}
