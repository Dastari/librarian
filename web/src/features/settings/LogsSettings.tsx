import { useMutation, useSubscription } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { IconFileText, IconPlayerPause, IconPlayerPlay, IconTrash } from "@tabler/icons-react";
import { useEffect, useMemo, useState } from "react";

import { Button, DataTable, type DataTableColumn, EmptyState, ErrorState, Panel, SegmentTabs, StatusChip } from "@/components/ui";
import { useConnection } from "@/features/libraries/browser/useConnection";
import { EntityAppLogChangedDocument, EntityAppLogDeleteManyDocument, EntityAppLogListDocument, type AppLogFieldsFragment, type AppLogWhereInput } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { formatDateTime } from "@/lib/format";
import { errorMessage } from "@/lib/graphql/errors";
import { cn } from "@/lib/utils";

const LEVELS = ["all", "ERROR", "WARN", "INFO", "DEBUG"] as const;
type Level = (typeof LEVELS)[number];
const LEVEL_TONE: Record<string, { tone: "danger" | "warning" | "accent" | "default"; dot: string }> = {
  ERROR: { tone: "danger", dot: "bg-danger" },
  WARN: { tone: "warning", dot: "bg-warning" },
  INFO: { tone: "accent", dot: "bg-info" },
  DEBUG: { tone: "default", dot: "bg-muted" },
  TRACE: { tone: "default", dot: "bg-muted" },
};

export function LogsSettings() {
  const [level, setLevel] = useState<Level>("all");
  const [live, setLive] = useState(true);
  const [expanded, setExpanded] = useState<string | null>(null);
  const table = useServerTable({ persistKey: "logs", defaultPageSize: 100 });
  useEffect(() => table.resetPage(), [level, table.resetPage]);
  const where = useMemo<AppLogWhereInput>(() => (level === "all" ? {} : { level: { eq: level } }), [level]);
  const list = useConnection(EntityAppLogListDocument, { where, orderBy: [{ timestamp: "DESC" }], page: table.page }, (data) => data.appLogs);
  useSubscription(EntityAppLogChangedDocument, { skip: !live || table.pageIndex > 0, onData: () => void list.refetch() });
  const [clear, { loading: clearing }] = useMutation(EntityAppLogDeleteManyDocument);

  const columns: Array<DataTableColumn<AppLogFieldsFragment>> = [
    { id: "time", header: "Time", size: 170, cell: (log) => <span className="text-numeric text-muted">{formatDateTime(log.timestamp)}</span> },
    { id: "level", header: "Level", size: 90, cell: (log) => <StatusChip minimal status={{ label: log.level, ...(LEVEL_TONE[log.level] ?? LEVEL_TONE.INFO!) }} /> },
    { id: "target", header: "Module", size: 220, hideBelow: "lg", cell: (log) => <span className="truncate font-mono text-label-sm text-muted">{log.target}</span> },
    { id: "message", header: "Message", cell: (log) => <span className={cn("font-mono text-label text-foreground", expanded === log.id ? "whitespace-pre-wrap" : "truncate")}>{log.message}{expanded === log.id && log.fields ? `\n${log.fields}` : ""}</span> },
  ];

  return (
    <Panel
      title="System log"
      flush
      actions={
        <>
          <Button size="sm" variant="ghost" onPress={() => setLive((value) => !value)}>
            {live ? <IconPlayerPause size={16} /> : <IconPlayerPlay size={16} />} {live ? "Live" : "Paused"}
          </Button>
          <Button size="sm" variant="danger-soft" isPending={clearing} onPress={async () => { try { await clear({ variables: { where: { level: { ne: "__never__" } } } }); toast.success("Log cleared"); void list.refetch(); } catch (error) { toast.danger(errorMessage(error)); } }}>
            <IconTrash size={16} /> Clear
          </Button>
        </>
      }
    >
      <div className="px-4 pb-4">
        <SegmentTabs ariaLabel="Log level" size="sm" items={LEVELS.map((item) => ({ key: item, label: item === "all" ? "All" : item.charAt(0) + item.slice(1).toLowerCase() }))} selected={level} onSelect={(key) => setLevel(key as Level)} className="mb-3" />
        <DataTable<AppLogFieldsFragment>
          frame={false}
          columns={columns}
          rows={list.rows}
          getRowId={(log) => log.id}
          isLoading={list.loading}
          totalCount={list.totalCount}
          hasNextPage={list.hasNextPage}
          density="compact"
          noun="entries"
          {...table.tableProps}
          sorting={[]}
          onSortingChange={undefined}
          onRowClick={(log) => setExpanded((current) => (current === log.id ? null : log.id))}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} compact /> : undefined}
          emptyState={<EmptyState compact icon={IconFileText} title="No log entries" />}
        />
      </div>
    </Panel>
  );
}
