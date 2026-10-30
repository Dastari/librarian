import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { Link } from "@tanstack/react-router";
import { IconArrowUpCircle, IconBell, IconCheck, IconChecks, IconTrash } from "@tabler/icons-react";
import { useMemo } from "react";
import { parseAsString, parseAsStringEnum, useQueryState } from "nuqs";
import { SearchField } from "glass-ux/input";

import { Button, DataTable, type DataTableBulkAction, type DataTableColumn, type DataTableRowAction, EmptyState, ErrorState, PageHeader, SegmentTabs, StatusChip } from "@/components/ui";
import { useInfiniteConnection } from "@/features/libraries/browser/useInfiniteConnection";
import {
  ApproveQualityUpgradeDocument,
  EntityNotificationDeleteDocument,
  EntityNotificationListDocument,
  EntityNotificationUpdateDocument,
  EntityNotificationUpdateManyDocument,
  type EntityNotificationListQuery,
  type NotificationWhereInput,
  type NotificationOrderByInput,
} from "@/graphql/generated/graphql";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
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
  const [tab, setTab] = useQueryState("show", parseAsStringEnum<Tab>([...TABS]).withDefault("unread").withOptions({ history: "push" }));
  const [query, setQuery] = useQueryState("q", parseAsString.withDefault(""));
  const search = useDebouncedValue(query.trim(), 250);
  const table = useServerTable<NotificationOrderByInput>({ persistKey: "activity", defaultSorting: [{ id: "createdAt", desc: true }] });

  const where = useMemo<NotificationWhereInput>(() => {
    const clause: NotificationWhereInput = {};
    if (tab === "unread") clause.readAt = { isNull: true };
    if (tab === "action") {
      clause.actionType = { isNull: false };
      clause.resolvedAt = { isNull: true };
    }
    if (search) clause.or = [{ title: { contains: search } }, { category: { contains: search } }];
    return clause;
  }, [tab, search]);
  const list = useInfiniteConnection(EntityNotificationListDocument, { where, orderBy: table.orderBy }, (data) => data.notifications);

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
                <span className={cn("block break-words text-body-sm", row.readAt ? "text-muted" : "text-foreground")}>{row.title}</span>
                <span className="block whitespace-pre-wrap break-words text-label-sm text-muted">{row.message}</span>
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
      { id: "createdAt", header: "When", size: 130, sortable: true, cell: (row) => <span className="text-muted">{formatRelative(row.createdAt)}</span> },
    ],
    [],
  );

  return (
    <div data-print-flow className="page-gutter flex min-h-0 flex-1 flex-col gap-4 overflow-hidden py-4 md:gap-6 md:py-6 [@media(max-height:35rem)]:gap-2 [@media(max-height:35rem)]:py-2">
      <PageHeader
        title="Activity"
        titleVisibility="accessible"
        actions={
          <Button variant="secondary" onPress={() => void markAllRead()}>
            <IconChecks size={16} /> Mark all read
          </Button>
        }
      />
      <SegmentTabs ariaLabel="Activity filter" items={[{ key: "unread", label: "Unread" }, { key: "action", label: "Needs action" }, { key: "all", label: "Everything" }]} selected={tab} onSelect={(key) => setTab(key as Tab)} />
      <SearchField aria-label="Filter activity" placeholder="Filter activity" value={query} onValueChange={(value) => void setQuery(value)} className="w-full shrink-0 sm:max-w-sm" />
      <DataTable<Row>
        height="fill"
        columns={columns}
        rows={list.rows}
        getRowId={(row) => row.id}
        isLoading={list.loading}
        refetching={list.fetching}
        queryKey={list.queryKey} loadRevision={list.loadRevision}
        externalBrowserToolbar
        totalCount={list.totalCount}
        infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
        rowActions={rowActions}
        bulkActions={bulkActions}
        selectable
        noun="notifications"
        sorting={table.sorting}
        onSortingChange={table.setSorting}
        onRowClick={(row) => (row.readAt ? undefined : void run(() => update({ variables: { id: row.id, input: { readAt: now() } } })))}
        error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
        emptyState={<EmptyState icon={IconBell} title={tab === "unread" ? "You're all caught up" : "Nothing here"} />}
      />
    </div>
  );
}
