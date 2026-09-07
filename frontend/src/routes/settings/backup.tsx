import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody, CardHeader } from "@heroui/card";
import { Chip } from "@heroui/chip";
import { Divider } from "@heroui/divider";
import { Tooltip } from "@heroui/tooltip";
import { addToast } from "@heroui/toast";
import {
  IconArchive,
  IconCircleCheck,
  IconCircleX,
  IconDatabaseExport,
  IconRefresh,
  IconShieldCheck,
} from "@tabler/icons-react";

import { Spinner } from "@heroui/spinner";
import { DataTable } from "../../components/data-table/DataTable";
import type { DataTableColumn, RowAction } from "../../components/data-table/types";
import { apolloClient } from "../../lib/graphql/client";
import {
  BackupSettingsDocument,
  CreateFullBackupDocument,
  VerifyBackupSnapshotDocument,
  type BackupSettingsQuery,
  type CreateFullBackupMutation,
  type VerifyBackupSnapshotMutation,
} from "../../lib/graphql/generated/graphql";
import { sanitizeError } from "../../lib/format";

export const Route = createFileRoute("/settings/backup")({
  component: BackupSettingsPage,
});

type BackupCapabilities = BackupSettingsQuery["backupCapabilities"];
type BackupSnapshot = BackupSettingsQuery["backupSnapshots"][number];

function BackupSettingsPage() {
  const [capabilities, setCapabilities] = useState<BackupCapabilities | null>(null);
  const [snapshots, setSnapshots] = useState<BackupSnapshot[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [verifyingId, setVerifyingId] = useState<string | null>(null);

  const loadSettings = useCallback(async () => {
    setIsLoading(true);
    try {
      const { data } = await apolloClient.query<BackupSettingsQuery>({
        query: BackupSettingsDocument,
        fetchPolicy: "network-only",
      });
      if (!data) return;
      setCapabilities(data.backupCapabilities);
      setSnapshots(data.backupSnapshots ?? []);
    } catch (err) {
      addToast({
        title: "Failed to load backup settings",
        description: sanitizeError(err),
        color: "danger",
      });
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  const createBackup = useCallback(async () => {
    setIsCreating(true);
    try {
      const { data } = await apolloClient.mutate<CreateFullBackupMutation>({
        mutation: CreateFullBackupDocument,
      });
      const result = data?.createFullBackup;
      if (!result?.success) {
        addToast({
          title: "Backup unavailable",
          description: result?.error ?? "Full database backup is not available",
          color: "warning",
        });
        return;
      }
      addToast({
        title: "Backup created",
        description: result.snapshot?.snapshotId ?? "Snapshot created",
        color: "success",
      });
      await loadSettings();
    } catch (err) {
      addToast({
        title: "Failed to create backup",
        description: sanitizeError(err),
        color: "danger",
      });
    } finally {
      setIsCreating(false);
    }
  }, [loadSettings]);

  const verifySnapshot = useCallback(async (snapshot: BackupSnapshot) => {
    setVerifyingId(snapshot.snapshotId);
    try {
      const { data } = await apolloClient.mutate<VerifyBackupSnapshotMutation>({
        mutation: VerifyBackupSnapshotDocument,
        variables: { snapshotId: snapshot.snapshotId },
      });
      const result = data?.verifyBackupSnapshot;
      addToast({
        title: result?.success ? "Snapshot verified" : "Snapshot verification failed",
        description: result?.error ?? snapshot.snapshotId,
        color: result?.success ? "success" : "danger",
      });
    } catch (err) {
      addToast({
        title: "Snapshot verification failed",
        description: sanitizeError(err),
        color: "danger",
      });
    } finally {
      setVerifyingId(null);
    }
  }, []);

  const columns = useMemo<DataTableColumn<BackupSnapshot>[]>(
    () => [
      {
        key: "snapshotId",
        label: "Snapshot ID",
        width: { minWidth: 260, grow: 2 },
        render: (snapshot) => (
          <span className="font-mono text-xs text-default-700">{snapshot.snapshotId}</span>
        ),
      },
      {
        key: "createdAt",
        label: "Created",
        width: 170,
        render: (snapshot) => formatTimestamp(snapshot.createdAt),
        sortFn: (a, b) => a.createdAt - b.createdAt,
      },
      { key: "kind", label: "Type", width: 120 },
      {
        key: "tableCount",
        label: "Tables",
        width: 100,
        align: "end",
      },
      {
        key: "objectCount",
        label: "Objects",
        width: 110,
        align: "end",
      },
      {
        key: "totalObjectBytes",
        label: "Size",
        width: 120,
        align: "end",
        render: (snapshot) => formatBytes(snapshot.totalObjectBytes),
        sortFn: (a, b) => a.totalObjectBytes - b.totalObjectBytes,
      },
      { key: "appVersion", label: "App version", width: 130 },
    ],
    [],
  );

  const rowActions = useMemo<RowAction<BackupSnapshot>[]>(
    () => [
      {
        key: "verify",
        label: "Verify",
        icon: <IconShieldCheck className="h-4 w-4" />,
        onAction: verifySnapshot,
        isDisabled: (snapshot) => verifyingId === snapshot.snapshotId,
      },
    ],
    [verifySnapshot, verifyingId],
  );

  const canCreateBackup = Boolean(capabilities?.fullDatabaseBackupAvailable);
  const createReason =
    capabilities?.reason ?? "Full database backup is not available in this build";

  if (isLoading && !capabilities) return <Spinner label="Loading backup settings" />;

  return (
      <div className="flex h-full min-h-0 flex-col gap-6">
        <div className="shrink-0">
          <h2 className="text-xl font-semibold">Backup</h2>
          <p className="mt-1 text-sm text-default-500">
            Create and verify database and object snapshots.
          </p>
        </div>

        <section className="grid shrink-0 gap-4 lg:grid-cols-4">
          <CapabilityCard label="Database" enabled={capabilities?.fullDatabaseBackupAvailable} />
          <CapabilityCard label="Objects" enabled={capabilities?.objectBackupAvailable} />
          <CapabilityCard label="Restore in app" enabled={false} />
          <CapabilityCard label="Incremental" enabled={capabilities?.incrementalBackupAvailable} />
        </section>

        {capabilities?.reason && (
          <Card className="shrink-0 border border-warning/30 bg-warning/10">
            <CardBody className="flex flex-row items-start gap-3 p-4 text-sm text-warning-700 dark:text-warning-300">
              <IconCircleX className="mt-0.5 h-5 w-5 shrink-0" />
              <span>{capabilities.reason}</span>
            </CardBody>
          </Card>
        )}

        <Card className="shrink-0">
          <CardHeader className="flex items-center justify-between gap-4">
            <div>
              <h3 className="font-semibold">Snapshot actions</h3>
              <p className="text-sm text-default-500">Create and verify local backup manifests.</p>
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant="flat"
                startContent={<IconRefresh className="h-4 w-4" />}
                onPress={() => void loadSettings()}
                isLoading={isLoading}
              >
                Refresh
              </Button>
              <Tooltip content={canCreateBackup ? "Create full backup" : createReason}>
                <span>
                  <Button
                    color="primary"
                    startContent={<IconDatabaseExport className="h-4 w-4" />}
                    isDisabled={!canCreateBackup}
                    isLoading={isCreating}
                    onPress={() => void createBackup()}
                  >
                    Create Backup
                  </Button>
                </span>
              </Tooltip>
            </div>
          </CardHeader>
        </Card>

        <section className="flex min-h-0 flex-1 flex-col">
          <DataTable
            data={snapshots}
            columns={columns}
            getRowKey={(snapshot) => snapshot.snapshotId}
            fillHeight
            rowActions={rowActions}
            defaultSortColumn="CreatedAt"
            defaultSortDirection="desc"
            toolbarQueryPlaceholder="Search snapshots..."
            isLoading={isLoading}
            emptyContent={
              <div className="flex flex-col items-center gap-2 py-12 text-center text-default-500">
                <IconArchive className="h-8 w-8" />
                <span>No backup snapshots found</span>
              </div>
            }
            classNames={{ wrapper: "h-full min-h-0" }}
            ariaLabel="Backup snapshots"
          />
        </section>

        <Card className="shrink-0">
          <CardBody className="gap-3 p-4 text-sm text-default-500">
            <div className="flex items-center gap-2 font-medium text-foreground">
              <IconArchive className="h-4 w-4 text-primary" />
              Restore status
            </div>
            <Divider />
            <p>
              Restoring a snapshot is not available in the app yet. Database recovery is
              supported by the backend, but restoring stored artwork remains incomplete.
            </p>
          </CardBody>
        </Card>
      </div>
  );
}

function CapabilityCard({
  label,
  enabled,
}: {
  label: string;
  enabled?: boolean;
}) {
  return (
    <Card>
      <CardBody className="flex flex-row items-center justify-between gap-3 p-4">
        <span className="text-sm font-medium">{label}</span>
        <Chip
          size="sm"
          color={enabled ? "success" : "default"}
          variant="flat"
          startContent={
            enabled ? <IconCircleCheck className="h-3.5 w-3.5" /> : <IconCircleX className="h-3.5 w-3.5" />
          }
        >
          {enabled ? "Available" : "Unavailable"}
        </Chip>
      </CardBody>
    </Card>
  );
}

function formatTimestamp(value: number) {
  return new Date(value * 1000).toLocaleString();
}

function formatBytes(value: number) {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }
  return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}
