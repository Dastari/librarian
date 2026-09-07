import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { IconDatabase, IconDatabaseExport, IconRestore, IconShieldCheck } from "@tabler/icons-react";
import { useState } from "react";

import { Button, ConfirmDialog, DataTable, type DataTableColumn, type DataTableRowAction, EmptyState, Panel, StatTile } from "@/components/ui";
import { BackupOverviewDocument, CreateFullBackupDocument, RestoreFullBackupDocument, VerifyBackupSnapshotDocument, type BackupOverviewQuery } from "@/graphql/generated/graphql";
import { formatBytes, formatDateTime } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

type Snapshot = BackupOverviewQuery["backupSnapshots"][number];

export function BackupSettings() {
  const { data, previousData, loading, refetch } = useQuery(BackupOverviewDocument);
  const overview = data ?? previousData;
  const snapshots = overview?.backupSnapshots ?? [];
  const capabilities = overview?.backupCapabilities;
  const [createBackup, { loading: creating }] = useMutation(CreateFullBackupDocument);
  const [verify] = useMutation(VerifyBackupSnapshotDocument);
  const [restore, { loading: restoring }] = useMutation(RestoreFullBackupDocument);
  const [restoreTarget, setRestoreTarget] = useState<Snapshot | null>(null);

  const create = async () => {
    try {
      const snapshot = assertSuccess((await createBackup()).data?.createFullBackup, "Backup failed").snapshot;
      toast.success(`Backup ${snapshot?.snapshotId ?? "created"}`);
      void refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const columns: Array<DataTableColumn<Snapshot>> = [
    { id: "id", header: "Snapshot", cell: (snapshot) => (<span className="min-w-0"><span className="block truncate font-mono text-label text-foreground">{snapshot.snapshotId}</span><span className="block truncate text-label-sm text-muted">{snapshot.kind} · v{snapshot.appVersion}</span></span>) },
    { id: "created", header: "Created", size: 170, cell: (snapshot) => formatDateTime(snapshot.createdAt) },
    { id: "tables", header: "Tables", size: 90, align: "end", numeric: true, hideBelow: "md", cell: (snapshot) => snapshot.tableCount },
    { id: "objects", header: "Objects", size: 100, align: "end", numeric: true, hideBelow: "md", cell: (snapshot) => snapshot.objectCount },
    { id: "size", header: "Size", size: 100, align: "end", numeric: true, cell: (snapshot) => formatBytes(snapshot.totalObjectBytes) },
  ];
  const actions: Array<DataTableRowAction<Snapshot>> = [
    { key: "verify", label: "Verify", icon: <IconShieldCheck size={16} />, onAction: async (snapshot) => { try { assertSuccess((await verify({ variables: { snapshotId: snapshot.snapshotId } })).data?.verifyBackupSnapshot, "Verification failed"); toast.success("Snapshot verified"); } catch (error) { toast.danger(errorMessage(error)); } } },
    { key: "restore", label: "Restore…", icon: <IconRestore size={16} />, destructive: true, hidden: () => !capabilities?.restoreAvailable, onAction: (snapshot) => setRestoreTarget(snapshot) },
  ];

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <StatTile label="Database backup" value={capabilities?.fullDatabaseBackupAvailable ? "Available" : "Unavailable"} tone={capabilities?.fullDatabaseBackupAvailable ? "success" : "warning"} />
        <StatTile label="Object backup" value={capabilities?.objectBackupAvailable ? "Available" : "Unavailable"} tone={capabilities?.objectBackupAvailable ? "success" : "warning"} />
        <StatTile label="Restore" value={capabilities?.restoreAvailable ? "Available" : "Unavailable"} tone={capabilities?.restoreAvailable ? "success" : "warning"} hint={capabilities?.reason ?? undefined} />
        <StatTile label="Snapshots" value={snapshots.length} icon={IconDatabase} />
      </div>
      <Panel title="Snapshots" description="Full backups of the catalogue database and stored objects such as artwork." flush actions={<Button size="sm" variant="primary" onPress={() => void create()} isPending={creating} isDisabled={!capabilities?.fullDatabaseBackupAvailable}><IconDatabaseExport size={16} /> Back up now</Button>}>
        <DataTable<Snapshot> className="px-4 pb-4" frame={false} columns={columns} rows={[...snapshots].sort((a, b) => b.createdAt - a.createdAt)} getRowId={(snapshot) => snapshot.snapshotId} isLoading={loading && snapshots.length === 0} rowActions={actions} noun="snapshots" emptyState={<EmptyState compact icon={IconDatabase} title="No snapshots yet" />} />
      </Panel>
      <ConfirmDialog isOpen={Boolean(restoreTarget)} onOpenChange={(open) => !open && setRestoreTarget(null)} title="Restore this snapshot?" description="The current catalogue is replaced with the snapshot's contents. Media files on disk are not touched. Restore only into an empty or freshly reinstalled server." confirmLabel="Restore" destructive isPending={restoring} onConfirm={async () => { if (!restoreTarget) return; try { assertSuccess((await restore({ variables: { snapshotId: restoreTarget.snapshotId } })).data?.restoreFullBackup, "Restore failed"); toast.success("Restore complete. Reload to see the restored catalogue."); setRestoreTarget(null); } catch (error) { toast.danger(errorMessage(error)); } }} />
    </div>
  );
}
