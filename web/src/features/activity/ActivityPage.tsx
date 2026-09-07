import { useMutation, useSubscription } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { IconArrowUpCircle, IconBell, IconCheck, IconChecks, IconTrash } from "@tabler/icons-react";
import { useEffect, useMemo, useState } from "react";

import { Button, DataTable, type DataTableBulkAction, type DataTableColumn, type DataTableRowAction, EmptyState, ErrorState, PageHeader, SegmentTabs, StatusChip } from "@/components/ui";
import { useConnection } from "@/features/libraries/browser/useConnection";
import {
  ApproveQualityUpgradeDocument,
  EntityNotificationChangedDocument,
  EntityNotificationDeleteDocument,
  EntityNotificationListDocument,
  EntityNotificationUpdateDocument,
  EntityNotificationUpdateManyDocument,
  type EntityNotificationListQuery,
  type NotificationWhereInput,
} from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatRelative } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { notificationType } from "@/lib/status";
import { cn } from "@/lib/utils";

type Row = EntityNotificationListQuery["notifications"]["edges"][number]["node"];
const TABS = ["unread", "action", "all"] as const;
type Tab = (typeof TABS)[number];

/** Notifications, upgrade approvals and anything that needs a decision. */
export function ActivityPage() {
  const isAdmin = useIsAdmin();
  const [tab, setTab] = useState<Tab>("unread");
  const table = useServerTable({ persistKey: "activity", defaultPageSize: 50 });
  useEffect(() => table.resetPage(), [tab, table.resetPage]);

  const where = useMemo<NotificationWhereInput>(() => {
    if (tab === "unread") return { readAt: { isNull: true } };
    if (tab === "action") return { actionType: { isNull: false }, resolvedAt: { isNull: true } };
    return {};
  }, [tab]);
  const list = useConnection(EntityNotificationListDocument, { where, orderBy: [{ createdAt: "DESC" }], page: table.page }, (data) => data.notifications);
  useSubscription(EntityNotificationChangedDocument, { onData: () => void list.refetch() });

  const [update] = useMutation(EntityNotificationUpdateDocument);
  const [updateMany] = useMutation(EntityNotificationUpdateManyDocument);
  const [remove] = useMutation(EntityNotificationDeleteDocument);
  const [approve] = useMutation(ApproveQualityUpgradeDocument);

  const run = async (task: () => Promise<unknown>, success?: string) => {
    try {
      await task();
      if (success) toast.success(success);
      void list.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };
  const now = () => new Date().toISOString();

  const markAllRead = () => run(() => updateMany({ variables: { where: { readAt: { isNull: true } }, input: { readAt: now() } } }), "Everything marked as read");

  const rowActions = useMemo<Array<DataTableRowAction<Row>>>(
    () => [
      { key: "read", label: "Mark read", icon: <IconCheck size={16} />, hidden: (row) => Boolean(row.readAt), onAction: (row) => run(() => update({ variables: { id: row.id, input: { readAt: now() } } })) },
      {
        key: "approve",
        label: "Approve upgrade",
        icon: <IconArrowUpCircle size={16} />,
        hidden: (row) => !isAdmin || row.actionType !== "quality_upgrade" || Boolean(row.resolvedAt),
        onAction: (row) => run(async () => assertSuccess((await approve({ variables: { notificationId: row.id } })).data?.approveQualityUpgrade, "Could not approve"), "Upgrade approved"),
      },
      { key: "resolve", label: "Mark resolved", icon: <IconChecks size={16} />, hidden: (row) => !row.actionType || Boolean(row.resolvedAt), onAction: (row) => run(() => update({ variables: { id: row.id, input: { resolvedAt: now(), readAt: row.readAt ?? now(), resolution: "dismissed" } } }), "Resolved") },
      { key: "delete", label: "Delete", icon: <IconTrash size={16} />, destructive: true, onAction: (row) => run(() => remove({ variables: { id: row.id } })) },
    ],
    [approve, isAdmin, remove, update],
  );

  const bulkActions = useMemo<Array<DataTableBulkAction<Row>>>(
    () => [
      { key: "read", label: "Mark read", icon: <IconCheck size={16} />, onAction: (rows) => run(() => updateMany({ variables: { where: { id: { inList: rows.map((row) => row.id) } }, input: { readAt: now() } } })) },
      { key: "delete", label: "Delete", icon: <IconTrash size={16} />, destructive: true, onAction: (rows) => run(async () => Promise.all(rows.map((row) => remove({ variables: { id: row.id } })))) },
    ],
    [remove, updateMany],
  );

  const columns = useMemo<Array<DataTableColumn<Row>>>(
    () => [
      {
        id: "title",
        header: "Notification",
        cell: (row) => {
          const type = notificationType(row.notificationType);
          return (
            <span className="flex min-w-0 items-start gap-3">
              <span className={cn("mt-1.5 size-2 shrink-0 rounded-full", row.readAt ? "bg-transparent" : type.dot)} />
              <span className="min-w-0">
                <span className={cn("block truncate text-body-sm", row.readAt ? "text-muted" : "text-foreground")}>{row.title}</span>
                <span className="block truncate text-label-sm text-muted">{row.message}</span>
                {row.libraryId ? (
                  <Link to="/libraries/$libraryId" params={{ libraryId: row.libraryId }} className="nav-focus mt-0.5 inline-block rounded text-label-sm text-brand hover:underline" onClick={(event) => event.stopPropagation()}>
                    Open library
                  </Link>
                ) : null}
              </span>
            </span>
          );
        },
      },
      { id: "category", header: "Category", size: 150, hideBelow: "md", cell: (row) => <span className="capitalize text-muted">{row.category.toLowerCase().replace(/_/g, " ")}</span> },
      { id: "type", header: "Level", size: 130, cell: (row) => <StatusChip status={notificationType(row.notificationType)} /> },
      { id: "when", header: "When", size: 130, cell: (row) => <span className="text-muted">{formatRelative(row.createdAt)}</span> },
    ],
    [],
  );

  return (
    <div className="page-gutter flex flex-col gap-6 py-8">
      <PageHeader
        title="Activity"
        actions={
          <Button variant="secondary" onPress={() => void markAllRead()}>
            <IconChecks size={16} /> Mark all read
          </Button>
        }
      />
      <SegmentTabs ariaLabel="Activity filter" items={[{ key: "unread", label: "Unread" }, { key: "action", label: "Needs action" }, { key: "all", label: "Everything" }]} selected={tab} onSelect={(key) => setTab(key as Tab)} />
      <DataTable<Row>
        columns={columns}
        rows={list.rows}
        getRowId={(row) => row.id}
        isLoading={list.loading}
        totalCount={list.totalCount}
        hasNextPage={list.hasNextPage}
        rowActions={rowActions}
        bulkActions={bulkActions}
        selectable
        noun="notifications"
        {...table.tableProps}
        sorting={[]}
        onSortingChange={undefined}
        onRowClick={(row) => (row.readAt ? undefined : void run(() => update({ variables: { id: row.id, input: { readAt: now() } } })))}
        error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
        emptyState={<EmptyState icon={IconBell} title={tab === "unread" ? "You're all caught up" : "Nothing here"} />}
      />
    </div>
  );
}
