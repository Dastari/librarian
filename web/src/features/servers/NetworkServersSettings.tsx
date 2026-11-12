import { Badge } from "glass-ux/badge";
import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconPlus } from "glass-ux/icons";
import { useCallback, useState } from "react";

import { Button } from "@/components/ui";
import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { SettingsLoadError } from "@/features/settings/SettingsLoadError";
import { SettingsPage, SettingsSection } from "@/features/settings/SettingsSection";
import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import { EntityNetworkServerListDocument, type NetworkServerFieldsFragment } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { parseTimestamp } from "@/lib/format";

import { AddNetworkServerDialog, NetworkServerSettingsDialog } from "./NetworkServerDialogs";
import { NetworkServerIdentity, networkServerState, signInLabel } from "./NetworkServerIdentity";
import { useNetworkServerActions, type NetworkServerSettingsTab } from "./useNetworkServerActions";

type Server = NetworkServerFieldsFragment;

const column = createDataTableColumns<Server>();
const columns = column.columns([
  column.accessor("name", {
    header: "Server", width: 30, priority: 1, cardRole: "title",
    cell: ({ row }) => <NetworkServerIdentity server={row.original} />,
  }),
  column.accessor((row) => row.knownShares.join(", "), { id: "shares", header: "Shares", width: 18, priority: 2, cell: ({ row }) => <TextCell muted={row.original.knownShares.length === 0}><span className="font-mono">{row.original.knownShares.join(", ") || "None known yet"}</span></TextCell> }),
  column.accessor((row) => row.username ?? "", { id: "user", header: "Signs in as", width: 12, priority: 3, cell: ({ row }) => <TextCell muted={!row.original.username}>{row.original.username || signInLabel(row.original)}</TextCell> }),
  column.display({ id: "status", header: "Status", width: 12, priority: 2, cardRole: "status", cell: ({ row }) => { const state = networkServerState(row.original); return <Badge size="sm" marker="dot" tone={state.tone}>{state.label}</Badge>; } }),
  column.accessor((row) => row.lastCheckedAt ?? "", { id: "checked", header: "Last checked", width: 10, priority: 4, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.lastCheckedAt)} fallback="Never" /> }),
]);

/**
 * File servers Librarian signs in to itself: each saved once with its sign-in, so a library or the
 * download folder picks a share on it without retyping anything.
 */
export function NetworkServersSettings() {
  const { user } = useSession();
  const list = useSettingsConnection(EntityNetworkServerListDocument, { orderBy: [{ name: "ASC" }] }, (data) => data.networkServers);
  const { rows, refetch } = list;
  const [adding, setAdding] = useState(false);
  const [editing, setEditing] = useState<{ id: string; tab: NetworkServerSettingsTab } | null>(null);
  const refresh = useCallback(() => void refetch(), [refetch]);
  const open = useCallback((server: Server, tab?: NetworkServerSettingsTab) => setEditing({ id: server.id, tab: tab ?? "connection" }), []);
  const actions = useNetworkServerActions<Server>({ settings: open, onChanged: refresh });
  const current = editing ? rows.find((row) => row.id === editing.id) ?? null : null;

  return (
    <SettingsPage>
      <SettingsSection
        title="Network shares"
        description="File servers Librarian signs in to by itself, with nothing mounted. Save a server once; libraries and the download folder then choose a share on it. Passwords are stored encrypted and never shown again."
        actions={<Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus /> Add network share</Button>}
      >
        <SettingsLoadError error={list.error} retry={refetch} hasRows={rows.length > 0} />
        <LoadedDataTable<Server> complete={!list.loading && !list.error} height="auto" selectable queryKey={user?.id}
          id={`librarian.${user?.id ?? "session"}.settings.network-servers`}
          label="Network servers"
          columns={columns}
          rows={rows}
          getRowId={(row) => row.id}
          getRowLabel={(row) => row.name}
          loading={list.loading && rows.length === 0}
          actions={actions}
          renderBulkActions={(selected) => <TableBulkActions set={actions} rows={selected} noun={{ one: "server", other: "servers" }} />}
          onRowOpen={(row) => open(row)}
          rowOpen="click"
          noun={{ one: "server", other: "servers" }}
          emptyMessage="No servers saved yet. Add the file server your media or downloads are on, with its sign-in."
          emptyAction={<Button variant="primary" onPress={() => setAdding(true)}><IconPlus /> Add network share</Button>}
        />
      </SettingsSection>
      <AddNetworkServerDialog open={adding} onOpenChange={setAdding} onCreated={refresh} />
      {current && (
        <NetworkServerSettingsDialog
          server={current}
          open={editing !== null}
          onOpenChange={(next) => !next && setEditing(null)}
          tab={editing?.tab ?? "connection"}
          onTabChange={(tab) => setEditing((state) => (state ? { ...state, tab } : state))}
          onChanged={refresh}
        />
      )}
    </SettingsPage>
  );
}
