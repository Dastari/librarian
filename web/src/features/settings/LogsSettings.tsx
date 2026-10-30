import { useMutation } from "@apollo/client/react";
import { AlertDialog } from "glass-ux/alert-dialog";
import { Badge, type BadgeTone } from "glass-ux/badge";
import { createDataTableColumns, DataTable, filters, RelativeTime, TextCell, type DataTableSearch, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { Dialog } from "glass-ux/dialog";
import { IconFileText, IconPause, IconPlay, IconRefresh, IconTrash } from "glass-ux/icons";
import { useMemo, useRef, useState } from "react";

import { Button } from "@/components/ui";
import { useSettingsConnection } from "./useSettingsConnection";
import { EntityAppLogDeleteManyDocument, EntityAppLogKeysetDocument, EntityAppLogListDocument, type AppLogFieldsFragment } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { assertSuccess } from "@/lib/graphql/errors";
import { changeLogTableState, initialLogTableState, LOG_LEVELS, logTableVariables } from "@/lib/graphql/glass-table";
import { usePref } from "@/lib/prefs";
import { tableUrlFilters, useTableUrlState } from "@/hooks/useTableUrlState";
import { usePrintMode } from "@/hooks/usePrintMode";

const LEVEL_TONE: Record<string, BadgeTone> = { ERROR: "danger", WARN: "warning", INFO: "info", DEBUG: "neutral", TRACE: "neutral" };
const column = createDataTableColumns<AppLogFieldsFragment>();
const columns = column.columns([
  column.accessor("timestamp", { header: "Time", width: 11, priority: 2, cardRole: "subtitle", sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={row.original.timestamp} /> }),
  column.accessor("level", {
    header: "Level", width: 6, grow: false, priority: 2, cardRole: "status", sortable: true,
    filter: filters.options({ field: "level", nullable: false, operators: ["in", "notIn"], options: LOG_LEVELS.map((value) => ({ value, label: value })) }),
    cell: ({ row }) => <Badge size="sm" marker="dot" tone={LEVEL_TONE[row.original.level] ?? "neutral"}>{row.original.level}</Badge>,
  }),
  column.accessor("target", { header: "Module", width: 14, priority: 4, sortable: true, filter: filters.text({ field: "target", nullable: false, operators: ["contains", "eq", "ne"] }), cell: ({ row }) => <TextCell identifier muted>{row.original.target}</TextCell> }),
  column.accessor("message", { header: "Message", width: 28, priority: 1, cardRole: "title", truncate: "wrap", filter: filters.text({ field: "message", nullable: false, operators: ["contains", "eq", "ne"] }) }),
  column.accessor("id", { header: "Entry ID", width: 16, priority: 5, cardRole: "hidden", cell: ({ row }) => <TextCell identifier>{row.original.id}</TextCell> }),
]);
const search: DataTableSearch<AppLogFieldsFragment> = {
  fields: ["message", "target"], placeholder: "Search message or module",
  toCondition: (query) => ({ or: [{ message: { contains: query } }, { target: { contains: query } }] }),
};
const logUrlSchema = {
  columns,
  sortableColumns: ["timestamp", "level", "target"],
  filters: { level: tableUrlFilters.options(LOG_LEVELS), target: tableUrlFilters.text(), message: tableUrlFilters.text() },
};
const hiddenColumns = ["id"];
const getRowId = (log: AppLogFieldsFragment) => log.id;
const getRowLabel = (log: AppLogFieldsFragment) => log.message;
const noun = { one: "entry", other: "entries" };

export function LogsSettings() {
  const { user } = useSession();
  const [pageSize] = usePref<number>("table.logs.pageSize", 100);
  const printing = usePrintMode();
  const initial = useMemo(() => initialLogTableState(pageSize), [pageSize]);
  const url = useTableUrlState({ namespace: "settings.logs", ...logUrlSchema, defaultSorting: initial.sorting });
  const state = useMemo(() => ({ ...initial, ...url.state }), [initial, url.state]);
  const [live, setLive] = useState(true);
  const [entry, setEntry] = useState<AppLogFieldsFragment | null>(null);
  const focusReturn = useRef<HTMLElement | null>(null);
  const variables = useMemo(() => logTableVariables(state), [state]);
  const defaultOrder = state.sorting.length === 0 || (state.sorting.length === 1 && state.sorting[0]?.id === "timestamp" && state.sorting[0]?.desc === true);
  const cursor = useSettingsConnection(EntityAppLogKeysetDocument, { where: variables.where }, (data) => data.appLogsKeyset, { infinite: true, keyset: true, batchSize: variables.page?.limit ?? 100, live, skip: !defaultOrder });
  const offset = useSettingsConnection(EntityAppLogListDocument, { where: variables.where, orderBy: variables.orderBy }, (data) => data.appLogs, { infinite: true, batchSize: variables.page?.limit ?? 100, live, skip: Boolean(defaultOrder) });
  const list = defaultOrder ? cursor : offset;
  const [clear, { loading: clearing }] = useMutation(EntityAppLogDeleteManyDocument);

  const onStateChange = (next: DataTableState, change: DataTableStateChange) => {
    const changed = changeLogTableState(next, change);
    url.onStateChange(changed, change);
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex min-h-0 flex-1 flex-col">
        <DataTable<AppLogFieldsFragment>
          id={`librarian.${user?.id ?? "session"}.settings.logs`}
          label="System log"
          queryKey={JSON.stringify([user?.id, variables.where, variables.orderBy])}
          height="fill"
          columns={columns}
          defaultHiddenColumns={hiddenColumns}
          rows={list.rows}
          getRowId={getRowId}
          getRowLabel={getRowLabel}
          loading={list.loading}
          refetching={list.fetching && !list.loading}
          rowCount={list.totalCount}
          state={state}
          onStateChange={onStateChange}
          infinite={{ revision: list.revision, hasMore: !printing && list.hasMore, onLoadMore: list.loadMore, loadingMore: list.loadingMore, error: list.moreError?.message }}
          search={search}
          selectable={false}
          live={live}
          noun={noun}
          onRowOpen={(log) => { focusReturn.current = document.activeElement instanceof HTMLElement ? document.activeElement : null; setEntry(log); }}
          rowOpen="click"
          error={list.error && list.rows.length === 0 ? "Couldn't load log entries. Retry to reconnect." : undefined}
          onRetry={() => { void list.refetch().catch(() => {}); }}
          emptyMessage="No log entries"
          status={list.error && list.rows.length > 0 ? "Refresh failed. Showing loaded entries." : !live ? "Paused" : "Live"}
          toolbar={
            <>
              <Button size="sm" variant="ghost" aria-pressed={live} onPress={() => setLive((value) => !value)}>
                {live ? <IconPause /> : <IconPlay />} {live ? "Live" : "Paused"}
              </Button>
              <Button size="sm" variant="ghost" isIconOnly aria-label="Refresh system log" isPending={list.fetching} onPress={() => { void list.refetch().catch(() => {}); }}><IconRefresh /></Button>
              <AlertDialog
                trigger={<Button size="sm" variant="danger-soft" aria-label="Clear system log" isPending={clearing}><IconTrash /> Clear</Button>}
                title="Clear all log entries?"
                description="This permanently deletes all system log entries, including entries outside the current filters. This cannot be undone."
                confirmLabel="Clear all logs"
                pendingLabel="Clearing logs"
                onConfirm={async () => {
                  // IDs are non-null. Use an explicit non-empty predicate for bulk deletion.
                  const result = await clear({ variables: { where: { id: { isNull: false } } } });
                  assertSuccess(result.data?.deleteAppLogs, "Couldn't clear system logs");
                  setEntry(null);
                  // Discard loaded batches after clearing, even if the reader was far down the feed.
                  void list.refetch({ reset: true });
                }}
              />
            </>
          }
        />
        <Dialog.Root open={entry !== null} onOpenChange={(open) => { if (!open) setEntry(null); }}>
          <Dialog.Popup size="lg" finalFocus={focusReturn}>
            <Dialog.Header title="Log entry" icon={<IconFileText />} subtitle={entry?.timestamp} />
            <Dialog.Body>
              {entry ? <div className="flex flex-col gap-4" data-spatial-ignore>
                <div className="flex flex-wrap items-center gap-2"><Badge size="sm" marker="dot" tone={LEVEL_TONE[entry.level] ?? "neutral"}>{entry.level}</Badge><TextCell identifier>{entry.target}</TextCell></div>
                <p className="whitespace-pre-wrap break-words [overflow-wrap:anywhere] font-mono">{entry.message}</p>
                {entry.fields ? <pre className="whitespace-pre-wrap break-words [overflow-wrap:anywhere]">{entry.fields}</pre> : null}
                {entry.spanName ? <p>Span: {entry.spanName}{entry.spanId ? ` (${entry.spanId})` : ""}</p> : null}
              </div> : null}
            </Dialog.Body>
          </Dialog.Popup>
        </Dialog.Root>
      </div>
    </div>
  );
}
