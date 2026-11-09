import { Badge } from "glass-ux/badge";
import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";
import { useCallback, useMemo, useState, useSyncExternalStore } from "react";

import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { currentDevice } from "@/features/devices/clientDevice";
import { DeviceIdentity, deviceSoftware, deviceState } from "@/features/devices/DeviceIdentity";
import { DeviceSettingsDialog } from "@/features/devices/DeviceSettingsDialog";
import { useDeviceActions, type DeviceSettingsTab } from "@/features/devices/useDeviceActions";
import { SettingsLoadError } from "@/features/settings/SettingsLoadError";
import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import { DashboardDevicesDocument, EntityUserListDocument, type DeviceFieldsFragment } from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { parseTimestamp } from "@/lib/format";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

type Row = DeviceFieldsFragment;

function columnsFor(current: string | null, people: ReadonlyMap<string, string> | null) {
  const column = createDataTableColumns<Row>();
  return column.columns([
    column.accessor("name", {
      header: "Device", width: 30, priority: 1, cardRole: "title",
      cell: ({ row }) => <DeviceIdentity device={row.original} current={row.original.id === current} owner={people ? people.get(row.original.userId) ?? "Removed user" : undefined} />,
    }),
    ...(people ? [column.accessor((row) => people.get(row.userId) ?? "", { id: "person", header: "Person", width: 12, priority: 2, cell: ({ row }) => <TextCell>{people.get(row.original.userId) ?? "Removed user"}</TextCell> })] : []),
    column.accessor((row) => deviceSoftware(row), { id: "software", header: "Browser and platform", width: 16, priority: 3, cell: ({ row }) => <TextCell muted>{deviceSoftware(row.original) || "—"}</TextCell> }),
    column.display({
      id: "status", header: "Status", width: 10, priority: 2, cardRole: "status",
      cell: ({ row }) => { const state = deviceState(row.original, { current: row.original.id === current }); return <Badge size="sm" marker="dot" tone={state.tone}>{state.label}</Badge>; },
    }),
    column.accessor((row) => row.lastIp ?? "", { id: "address", header: "Last address", width: 12, priority: 4, cell: ({ row }) => <TextCell muted><span className="font-mono">{row.original.lastIp ?? "—"}</span></TextCell> }),
    column.accessor((row) => row.appVersion ?? "", { id: "version", header: "App version", width: 8, priority: 5, cell: ({ row }) => <TextCell muted>{row.original.appVersion ?? "—"}</TextCell> }),
    column.accessor((row) => row.lastSeenAt, { id: "seen", header: "Last seen", width: 9, priority: 2, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.lastSeenAt)} /> }),
  ]);
}

const noun = { one: "device", other: "devices" };

/**
 * The browsers, apps and TVs signed in to Librarian: yours, or everyone's for an administrator.
 * A device can be renamed, signed out (its login ends and its playback stops) or forgotten.
 */
export function DevicesPage() {
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const current = useSyncExternalStore(currentDevice.subscribe, currentDevice.get, currentDevice.get);
  const users = useQuery(EntityUserListDocument, { variables: { page: { limit: 100, offset: 0 } }, skip: !isAdmin });
  const people = useMemo(() => (isAdmin ? new Map((users.data?.users.edges ?? []).map(({ node }) => [node.id, node.displayName || node.username])) : null), [isAdmin, users.data]);
  const list = useSettingsConnection(DashboardDevicesDocument, {}, (data) => data.devices);
  const { rows, refetch } = list;
  const refresh = useCallback(() => void refetch(), [refetch]);
  const [editing, setEditing] = useState<{ id: string; tab: DeviceSettingsTab } | null>(null);
  const open = useCallback((device: Row, tab?: DeviceSettingsTab) => setEditing({ id: device.id, tab: tab ?? "general" }), []);
  const actions = useDeviceActions<Row>({ settings: open, onChanged: refresh });
  const columns = useMemo(() => columnsFor(current, people), [current, people]);
  const editingRow = editing ? rows.find((row) => row.id === editing.id) ?? null : null;

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      <SettingsLoadError error={list.error} retry={refetch} hasRows={rows.length > 0} />
      <LoadedDataTable<Row>
        complete={!list.loading && !list.error}
        height="fill"
        selectable
        queryKey={user?.id}
        id={`librarian.${user?.id ?? "session"}.devices`}
        label="Devices"
        columns={columns}
        defaultHiddenColumns={["address", "version"]}
        rows={rows}
        getRowId={(row) => row.id}
        getRowLabel={(row) => row.name}
        loading={list.loading && rows.length === 0}
        actions={actions}
        renderBulkActions={(selected) => <TableBulkActions set={actions} rows={selected} noun={noun} />}
        onRowOpen={(row) => open(row)}
        rowOpen="click"
        noun={noun}
        emptyMessage="No devices yet. Each browser, app or TV that signs in appears here."
      />
      {editingRow ? (
        <DeviceSettingsDialog
          device={editingRow}
          owner={people?.get(editingRow.userId)}
          open={editing !== null}
          onOpenChange={(next) => !next && setEditing(null)}
          tab={editing?.tab ?? "general"}
          onTabChange={(tab) => setEditing((state) => (state ? { ...state, tab } : state))}
          onChanged={refresh}
        />
      ) : null}
    </div>
  );
}
