import { defineActions, type ActionSet } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, filters, IdentityCell, NumericCell, RelativeTime, TextCell, type DataTableSearch, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { IconClose, IconFilter, IconHistory, IconRefresh } from "glass-ux/icons";
import { MeterCell } from "glass-ux/meter-cell";
import { Metric } from "glass-ux/metric";
import { Segmented } from "glass-ux/segmented";
import { parseAsString, parseAsStringEnum, useQueryState } from "nuqs";
import { useCallback, useMemo, useState } from "react";

import { EntityChip } from "@/components/entities/EntityChip";
import { Button, Dialog } from "@/components/ui";
import { useMediaQuery } from "@/hooks/useMediaQuery";
import { usePrintMode } from "@/hooks/usePrintMode";
import { tableUrlFilters, useTableUrlState } from "@/hooks/useTableUrlState";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { ENTITY_KINDS, type EntityKindName } from "@/lib/entities/kinds";
import { linkedItem, mediaFileRef } from "@/lib/entities/linked";
import { issueLabel, scanDuration, scanRunStatus, scanSeverity, scanStage, SCAN_RUN_STATUSES, SCAN_SEVERITIES, SCAN_STAGES } from "@/lib/entities/scan";
import { formatDateTime, parseTimestamp } from "@/lib/format";
import { cn } from "@/lib/utils";

import { ScanIssueDetail } from "./scans/ScanIssueDetail";
import { ScanRunDetail } from "./scans/ScanRunDetail";
import { SCAN_VIEWS, useScanCounts, useScanIssues, useScanRuns, type IssueRow, type RunRow, type ScanView } from "./scans/useScans";
import { useScanIssueActions } from "./scans/useScanIssueActions";

const VIEW_LABEL: Record<ScanView, string> = { open: "Open issues", resolved: "Resolved", history: "Scan history" };
/** Issue codes the scanner writes (library_scan.rs); the filter lists them by name. */
const ISSUE_CODES = [
  "BYTE_IDENTICAL_DUPLICATE", "DUPLICATE_HASH_FAILED", "NOT_ANALYZED", "FFPROBE_UNAVAILABLE", "FFPROBE_TIMEOUT", "FFPROBE_MALFORMED_OUTPUT", "FFPROBE_OUTPUT_TOO_LARGE",
  "MEDIA_UNSUPPORTED_OR_CORRUPT", "MEDIA_PERMISSION_DENIED", "ANALYSIS_QUEUE_FAILED", "ANALYSIS_SETTLE_TIMEOUT", "ANALYSIS_INTERNAL_ERROR", "MATCH_PIPELINE_FAILED",
  "PROVIDER_REQUEST_FAILED", "TMDB_NOT_CONFIGURED", "FILE_DISCOVERY_ERROR", "LIBRARY_ROOT_UNAVAILABLE", "QUEUE_UNAVAILABLE", "SCAN_FAILED", "SCAN_CANCELLED",
];
const IssueGlyph = ENTITY_KINDS.scanIssue.icon;

function issueColumns() {
  const column = createDataTableColumns<IssueRow>();
  return column.columns([
    column.accessor((row) => row.issueCode, {
      id: "code", header: "Issue", width: 22, priority: 1, cardRole: "title", sortable: true, pin: true,
      filter: filters.options({ field: "issueCode", nullable: false, operators: ["in"], options: ISSUE_CODES.map((value) => ({ value, label: issueLabel(value) })) }),
      cell: ({ row }) => {
        const issue = row.original;
        const severity = scanSeverity(issue.severity);
        const item = issue.mediaFile ? linkedItem(issue.mediaFile) : null;
        const Glyph = item ? ENTITY_KINDS[item.kind as EntityKindName]?.icon ?? IssueGlyph : issue.mediaFile ? ENTITY_KINDS.mediaFile.icon : IssueGlyph;
        return <IdentityCell icon={<Glyph />} title={issueLabel(issue.issueCode)} subtitle={item?.label ?? (issue.mediaFile ? mediaFileRef(issue.mediaFile).label : scanStage(issue.stage))} status={issue.resolvedAt ? undefined : severity.status} statusLabel={severity.label} />;
      },
    }),
    column.accessor((row) => row.severity, {
      id: "severity", header: "Severity", width: 8, priority: 2, cardRole: "status", sortable: true,
      filter: filters.options({ field: "severity", nullable: false, operators: ["in"], options: SCAN_SEVERITIES.map((value) => ({ value, label: scanSeverity(value).label })) }),
      cell: ({ row }) => { const severity = scanSeverity(row.original.severity); return <Badge size="sm" marker="dot" tone={severity.tone}>{severity.label}</Badge>; },
    }),
    column.display({
      id: "file", header: "File", width: 18, priority: 2, cardRole: "subtitle",
      cell: ({ row }) => (row.original.mediaFile ? <EntityChip entity={mediaFileRef(row.original.mediaFile)} /> : <TextCell muted>—</TextCell>),
    }),
    column.accessor((row) => row.message, { id: "message", header: "Details", width: 26, priority: 4, cardRole: "hidden", cell: ({ row }) => <TextCell muted>{row.original.message}</TextCell> }),
    column.accessor((row) => row.stage, {
      id: "stage", header: "Stage", width: 9, priority: 3, sortable: true,
      filter: filters.options({ field: "stage", nullable: false, operators: ["in"], options: SCAN_STAGES.map((value) => ({ value, label: scanStage(value) })) }),
      cell: ({ row }) => <TextCell muted>{scanStage(row.original.stage)}</TextCell>,
    }),
    column.accessor((row) => row.occurrenceCount, { id: "seen", header: "Seen", width: 5, priority: 4, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <NumericCell value={row.original.occurrenceCount} /> }),
    column.accessor((row) => row.createdAt, { id: "first", header: "First seen", width: 8, priority: 5, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.createdAt)} /> }),
    column.accessor((row) => row.updatedAt, { id: "last", header: "Last seen", width: 8, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.updatedAt)} /> }),
  ]);
}

function runColumns() {
  const column = createDataTableColumns<RunRow>();
  return column.columns([
    column.accessor((row) => row.startedAt ?? row.createdAt, {
      id: "started", header: "Scan", width: 20, priority: 1, cardRole: "title", sortable: true, sortDescFirst: true, pin: true,
      cell: ({ row }) => {
        const status = scanRunStatus(row.original.status);
        return <IdentityCell icon={<IconHistory />} title={formatDateTime(row.original.startedAt ?? row.original.createdAt)} subtitle={[scanDuration(row.original), row.original.errorCode ?? row.original.summary].filter(Boolean).join(" · ") || scanStage(row.original.currentStage)} status={status.status} statusLabel={status.label} />;
      },
    }),
    column.accessor((row) => row.status, {
      id: "status", header: "Status", width: 11, priority: 2, cardRole: "status", sortable: true,
      filter: filters.options({ field: "status", nullable: false, operators: ["in"], options: SCAN_RUN_STATUSES.map((value) => ({ value, label: scanRunStatus(value).label })) }),
      cell: ({ row }) => { const status = scanRunStatus(row.original.status); return <Badge size="sm" marker="dot" tone={status.tone}>{status.label}</Badge>; },
    }),
    column.accessor((row) => row.discoveredCount, { id: "found", header: "Files", width: 6, priority: 2, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <NumericCell value={row.original.discoveredCount} /> }),
    column.accessor((row) => row.matchedCount, {
      id: "matched", header: "Matched", width: 12, priority: 2, sortable: true, sortDescFirst: true,
      cell: ({ row }) => {
        const total = row.original.matchedCount + row.original.unmatchedCount;
        return total ? <MeterCell label="Matched" value={row.original.matchedCount / total} max={1} tone="data" warningAt={2} dangerAt={2} valueText={`${row.original.matchedCount} of ${total}`} /> : <TextCell muted>—</TextCell>;
      },
    }),
    column.accessor((row) => row.unmatchedCount, { id: "unmatched", header: "Unmatched", width: 7, priority: 3, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <NumericCell value={row.original.unmatchedCount} /> }),
    column.accessor((row) => row.missingCount, { id: "missing", header: "Missing", width: 6, priority: 4, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <NumericCell value={row.original.missingCount} /> }),
    column.display({
      id: "analysed", header: "Analysed", width: 12, priority: 4,
      cell: ({ row }) => {
        const { analysisSucceededCount: done, analysisFailedCount: failed } = row.original;
        return done + failed ? <MeterCell label="Analysed" value={done / (done + failed)} max={1} tone="data" warningAt={2} dangerAt={2} valueText={failed ? `${done} · ${failed} failed` : String(done)} /> : <TextCell muted>—</TextCell>;
      },
    }),
    column.display({ id: "issues", header: "Issues", width: 6, priority: 3, align: "end", grow: false, cell: ({ row }) => <NumericCell value={row.original.issues.pageInfo.totalCount ?? 0} /> }),
  ]);
}

const issueSearch: DataTableSearch<IssueRow> = { fields: [], placeholder: "Search issues", toCondition: (query) => ({ message: { contains: query } }) };
const runSearch: DataTableSearch<RunRow> = { fields: [], placeholder: "Search error codes", toCondition: (query) => ({ errorCode: { contains: query } }) };
const issueUrl = {
  sortableColumns: ["code", "severity", "stage", "seen", "first", "last"],
  filters: { code: tableUrlFilters.options(ISSUE_CODES, ["in"]), severity: tableUrlFilters.options(SCAN_SEVERITIES, ["in"]), stage: tableUrlFilters.options([...SCAN_STAGES], ["in"]) },
  defaultSorting: [{ id: "last", desc: true }],
};
const runUrl = {
  sortableColumns: ["started", "status", "found", "matched", "unmatched", "missing"],
  filters: { status: tableUrlFilters.options(SCAN_RUN_STATUSES, ["in"]) },
  defaultSorting: [{ id: "started", desc: true }],
};
const tableState = (state: Partial<DataTableState>): DataTableState => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: {}, sorting: [], filters: [], ...state }) as DataTableState;

/**
 * A library's scans: open issues, resolved issues and the scan history as one bounded table
 * at a time. Issues link the file and the item it belongs to; runs open a pipeline breakdown
 * and lead to the issues they found. The header trends the last scans' totals instead of a
 * strip of status cards (docs/entity-tables.md).
 */
export function ScansPanel({ libraryId }: { libraryId: string }) {
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const printing = usePrintMode();
  const wide = useMediaQuery("(min-width: 1280px)");
  const [view, setView] = useQueryState("view", parseAsStringEnum<ScanView>([...SCAN_VIEWS]).withDefault("open").withOptions({ history: "push" }));
  const [scanRunId, setScanRunId] = useQueryState("scan", parseAsString.withOptions({ history: "push" }));
  const counts = useScanCounts(libraryId);

  const issueCols = useMemo(() => issueColumns(), []);
  const runCols = useMemo(() => runColumns(), []);
  const issueTable = useTableUrlState({ namespace: `scan-issues`, columns: issueCols, ...issueUrl });
  const runTable = useTableUrlState({ namespace: `scan-runs`, columns: runCols, ...runUrl });
  const issueState = useMemo(() => tableState(issueTable.state), [issueTable.state]);
  const runState = useMemo(() => tableState(runTable.state), [runTable.state]);

  const issues = useScanIssues(libraryId, issueState, view, scanRunId);
  const runs = useScanRuns(libraryId, runState, view);
  const refetch = useCallback(() => {
    void issues.list.refetch().catch(() => {});
    void runs.list.refetch().catch(() => {});
    counts.refetch();
  }, [counts, issues.list, runs.list]);
  const issueActions = useScanIssueActions(refetch, isAdmin);
  const runActions = useMemo<ActionSet<RunRow>>(() => defineActions("scan-run", [
    {
      id: "show-issues", label: "Show issues from this scan", icon: IconFilter, group: "open", priority: 1,
      visible: (rows) => rows.length === 1 && (rows[0]!.issues.pageInfo.totalCount ?? 0) > 0,
      run: (rows) => { void setScanRunId(rows[0]!.id); void setView("open"); },
    },
  ]), [setScanRunId, setView]);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const history = view === "history";
  const selectedIssue = !history ? issues.list.rows.find((row) => row.id === selectedId) ?? null : null;
  const selectedRun = history ? runs.list.rows.find((row) => row.id === selectedId) ?? null : null;
  const selected = selectedIssue ?? selectedRun;
  const close = () => setSelectedId(null);
  const latest = counts.recent.at(-1);
  const filteredRun = scanRunId ? runs.list.rows.find((row) => row.id === scanRunId) : null;

  const detail = selectedIssue ? <ScanIssueDetail row={selectedIssue} actions={issueActions} onClose={close} />
    : selectedRun ? <ScanRunDetail row={selectedRun} actions={runActions} onClose={close} /> : null;
  const refresh = <Button size="sm" variant="ghost" isIconOnly aria-label="Refresh scans" isPending={history ? runs.list.fetching : issues.list.fetching} onPress={refetch}><IconRefresh /></Button>;

  return (
    <div data-print-flow className="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden">
      <div className="flex shrink-0 flex-wrap items-center gap-x-6 gap-y-3">
        <Segmented<ScanView>
          aria-label="Show"
          value={view}
          onValueChange={(value) => { setSelectedId(null); void setView(value === "open" ? null : value); }}
          options={SCAN_VIEWS.map((value) => ({ value, label: counts.views[value] !== undefined ? `${VIEW_LABEL[value]} ${counts.views[value]}` : VIEW_LABEL[value] }))}
        />
        {scanRunId && !history ? (
          <Badge tone="info" size="md">
            From the scan of {filteredRun ? formatDateTime(filteredRun.startedAt ?? filteredRun.createdAt) : "the selected run"}
            <button type="button" aria-label="Show issues from every scan" className="nav-focus -me-1 ms-1 rounded-full" onClick={() => void setScanRunId(null)}><IconClose size="xs" /></button>
          </Badge>
        ) : null}
        {latest ? (
          <div className="ms-auto flex min-w-0 items-center gap-3">
            <div className="h-12 w-60 max-lg:hidden"><Metric label="Files found" value={latest.discoveredCount} data={counts.recent.map((run) => run.discoveredCount)} range={`Last ${counts.recent.length} scans`} tone="info" better="none" /></div>
            <div className="h-12 w-60 max-lg:hidden"><Metric label="Unmatched" value={latest.unmatchedCount} data={counts.recent.map((run) => run.unmatchedCount)} range={`Last ${counts.recent.length} scans`} tone="warning" better="down" /></div>
          </div>
        ) : null}
      </div>
      <div data-print-flow className={cn("grid min-h-0 flex-1 grid-rows-[minmax(0,1fr)] gap-6 overflow-hidden", selected && wide && "xl:grid-cols-[minmax(0,3fr)_minmax(22rem,2fr)]")}>
        <div className="flex min-h-0 min-w-0 flex-col">
          {history ? (
            <DataTable<RunRow>
              id={`librarian.${user?.id ?? "session"}.scan-runs`}
              label="Scan history"
              queryKey={JSON.stringify([user?.id, runs.variables])}
              height="fill"
              columns={runCols}
              rows={runs.list.rows}
              getRowId={(row) => row.id}
              getRowLabel={(row) => `Scan ${formatDateTime(row.startedAt ?? row.createdAt)}`}
              loading={runs.list.loading}
              refetching={runs.list.fetching && !runs.list.loading}
              rowCount={runs.list.totalCount}
              state={runState}
              onStateChange={(next: DataTableState, change: DataTableStateChange) => runTable.onStateChange(next, change)}
              infinite={{ revision: runs.list.revision, hasMore: !printing && runs.list.hasMore, onLoadMore: runs.list.loadMore, loadingMore: runs.list.loadingMore, error: runs.list.moreError?.message }}
              search={runSearch}
              actions={runActions}
              live
              noun={{ one: "scan", other: "scans" }}
              onRowOpen={(row) => setSelectedId(row.id)}
              rowOpen="click"
              error={runs.list.error && runs.list.rows.length === 0 ? "Couldn't load the scan history. Retry to reconnect." : undefined}
              onRetry={refetch}
              emptyMessage="No scans yet. Scan the library to discover its files."
              toolbar={refresh}
            />
          ) : (
            <DataTable<IssueRow>
              id={`librarian.${user?.id ?? "session"}.scan-issues`}
              label={view === "open" ? "Open issues" : "Resolved issues"}
              queryKey={JSON.stringify([user?.id, issues.variables])}
              height="fill"
              columns={issueCols}
              defaultHiddenColumns={["message", "first"]}
              rows={issues.list.rows}
              getRowId={(row) => row.id}
              getRowLabel={(row) => issueLabel(row.issueCode)}
              loading={issues.list.loading}
              refetching={issues.list.fetching && !issues.list.loading}
              rowCount={issues.list.totalCount}
              state={issueState}
              onStateChange={(next: DataTableState, change: DataTableStateChange) => issueTable.onStateChange(next, change)}
              infinite={{ revision: issues.list.revision, hasMore: !printing && issues.list.hasMore, onLoadMore: issues.list.loadMore, loadingMore: issues.list.loadingMore, error: issues.list.moreError?.message }}
              search={issueSearch}
              actions={issueActions}
              selectable={isAdmin}
              live
              noun={{ one: "issue", other: "issues" }}
              onRowOpen={(row) => setSelectedId(row.id)}
              rowOpen="click"
              error={issues.list.error && issues.list.rows.length === 0 ? "Couldn't load scan issues. Retry to reconnect." : undefined}
              onRetry={refetch}
              emptyMessage={view === "open" ? "No open issues. The last scans found nothing that needs you." : "No resolved issues yet."}
              toolbar={refresh}
            />
          )}
        </div>
        {detail && wide ? <div data-print-flow className="scrollbar-thin min-h-0 overflow-y-auto">{detail}</div> : null}
      </div>
      <Dialog isOpen={Boolean(detail) && !wide} onOpenChange={(open) => !open && close()} title={history ? "Scan" : "Scan issue"} size="md">
        {detail}
      </Dialog>
    </div>
  );
}
