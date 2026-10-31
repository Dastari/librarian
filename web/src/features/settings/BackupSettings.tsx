import { useMutation } from "@apollo/client/react";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell, NumericCell, RelativeTime, TextCell } from "glass-ux/data-table";
import { IconDatabase, IconDatabaseExport, IconRestore, IconShieldCheck } from "@tabler/icons-react";
import { useMemo } from "react";

import { Button } from "@/components/ui";
import { BackupOverviewDocument, CreateFullBackupDocument, RestoreFullBackupDocument, VerifyBackupSnapshotDocument, type BackupOverviewQuery } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { glyph } from "@/lib/entities/kinds";
import { formatBytes, parseTimestamp } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { toast } from "glass-ux/toast";

import { SettingsLoadError } from "./SettingsLoadError";
import { SettingsSection } from "./SettingsSection";

type Snapshot = BackupOverviewQuery["backupSnapshots"][number];

const SnapshotGlyph = glyph(IconDatabase, "database");
const VerifyIcon = glyph(IconShieldCheck, "shield-check");
const RestoreIcon = glyph(IconRestore, "restore");
const column = createDataTableColumns<Snapshot>();
const columns = column.columns([
  column.accessor("snapshotId", { header: "Snapshot", width: 22, priority: 1, cardRole: "title", cell: ({ row }) => <IdentityCell icon={<SnapshotGlyph />} title={row.original.snapshotId} subtitle={`${row.original.kind} · v${row.original.appVersion}`} /> }),
  column.accessor("createdAt", { header: "Created", width: 9, priority: 2, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.createdAt)} /> }),
  column.accessor("tableCount", { header: "Tables", width: 6, priority: 3, align: "end", grow: false, cell: ({ row }) => <NumericCell value={row.original.tableCount} /> }),
  column.accessor("objectCount", { header: "Objects", width: 6, priority: 3, align: "end", grow: false, cell: ({ row }) => <NumericCell value={row.original.objectCount} /> }),
  column.accessor("totalObjectBytes", { header: "Size", width: 7, priority: 2, align: "end", sortable: true, sortDescFirst: true, cell: ({ row }) => <TextCell muted>{formatBytes(row.original.totalObjectBytes)}</TextCell> }),
]);

/** What this server can back up and restore, as one line of badges instead of status cards. */
function Capability({ label, available }: { label: string; available: boolean | undefined }) {
  return <Badge size="sm" marker="dot" tone={available ? "success" : "warning"}>{label} {available ? "available" : "unavailable"}</Badge>;
}

export function BackupSettings() {
  const { user } = useSession();
  const { data, previousData, loading, error, refetch } = useQuery(BackupOverviewDocument);
  const overview = data ?? previousData;
  const snapshots = useMemo(() => [...(overview?.backupSnapshots ?? [])].sort((a, b) => b.createdAt - a.createdAt), [overview]);
  const capabilities = overview?.backupCapabilities;
  const [createBackup, { loading: creating }] = useMutation(CreateFullBackupDocument);
  const [verify] = useMutation(VerifyBackupSnapshotDocument);
  const [restore] = useMutation(RestoreFullBackupDocument);

  const create = async () => {
    try {
      const snapshot = assertSuccess((await createBackup()).data?.createFullBackup, "Backup failed").snapshot;
      toast.success(`Backup ${snapshot?.snapshotId ?? "created"}`);
      void refetch();
    } catch (failure) {
      toast.danger(errorMessage(failure));
    }
  };

  const actions = useMemo(() => defineActions<Snapshot>("backup-snapshot", [
    {
      id: "verify", label: "Verify", icon: VerifyIcon, group: "manage", priority: 1, visible: (targets) => targets.length === 1,
      run: async (targets) => { assertSuccess((await verify({ variables: { snapshotId: targets[0]!.snapshotId } })).data?.verifyBackupSnapshot, "Verification failed"); },
      successMessage: () => "Snapshot verified",
    },
    {
      id: "restore", label: "Restore…", icon: RestoreIcon, group: "danger",
      visible: (targets) => targets.length === 1 && Boolean(capabilities?.restoreAvailable),
      confirm: () => ({ title: "Restore this snapshot?", body: "The current catalogue is replaced with the snapshot's contents. Media files on disk are not touched. Restore only into an empty or freshly reinstalled server.", confirmLabel: "Restore", tone: "danger" }),
      run: async (targets) => { assertSuccess((await restore({ variables: { snapshotId: targets[0]!.snapshotId } })).data?.restoreFullBackup, "Restore failed"); },
      successMessage: () => "Restore complete. Reload to see the restored catalogue.",
    },
  ]), [capabilities?.restoreAvailable, restore, verify]);

  return (
    <SettingsSection
      title="Snapshots"
      description="Full backups of the catalogue database and stored objects such as artwork."
      actions={<Button size="sm" variant="primary" onPress={() => void create()} isPending={creating} isDisabled={!capabilities?.fullDatabaseBackupAvailable}><IconDatabaseExport size={16} /> Back up now</Button>}
    >
      {capabilities ? (
        <div className="flex flex-wrap items-center gap-2">
          <Capability label="Database backup" available={capabilities.fullDatabaseBackupAvailable} />
          <Capability label="Object backup" available={capabilities.objectBackupAvailable} />
          <Capability label="Restore" available={capabilities.restoreAvailable} />
          {capabilities.reason ? <span className="text-label-sm text-muted">{capabilities.reason}</span> : null}
        </div>
      ) : null}
      <SettingsLoadError error={error} retry={() => refetch().catch(() => {})} hasRows={snapshots.length > 0} />
      <DataTable<Snapshot>
        id={`librarian.${user?.id ?? "session"}.settings.backups`}
        label="Snapshots"
        columns={columns}
        rows={snapshots}
        getRowId={(snapshot) => snapshot.snapshotId}
        getRowLabel={(snapshot) => `Snapshot ${snapshot.snapshotId}`}
        loading={loading && snapshots.length === 0}
        actions={actions}
        noun={{ one: "snapshot", other: "snapshots" }}
        emptyMessage="No snapshots yet."
      />
    </SettingsSection>
  );
}
