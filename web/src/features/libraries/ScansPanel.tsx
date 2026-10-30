import { useMutation } from "@apollo/client/react";
import { SearchField } from "glass-ux/input";
import { Select } from "glass-ux/select";
import { toast } from "glass-ux/toast";
import { IconCheck, IconHistory, IconRefresh, IconTrash } from "@tabler/icons-react";
import { useMemo } from "react";
import { parseAsBoolean, parseAsString, parseAsStringEnum, useQueryState } from "nuqs";

import { Button, DataTable, type DataTableColumn, type DataTableRowAction, EmptyState, ErrorState, Panel, StatTile, StatusChip } from "@/components/ui";
import {
  EntityLibraryScanIssueListDocument,
  EntityLibraryScanRunListDocument,
  ResolveScanIssueDocument,
  RetryScanIssueDocument,
  TrashDuplicateScanIssueDocument,
  type LibraryScanIssuesQuery,
  type LibraryScanRunsQuery,
  type LibraryScanIssueWhereInput,
  type LibraryScanIssueOrderByInput,
  type LibraryScanRunOrderByInput,
} from "@/graphql/generated/graphql";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import { useServerTable } from "@/hooks/useServerTable";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatDateTime, formatRelative } from "@/lib/format";
import { errorMessage } from "@/lib/graphql/errors";
import { scanStatus } from "@/lib/status";

import { useInfiniteConnection } from "./browser/useInfiniteConnection";
import { useLibraryScan } from "./useLibraryScan";

type RunRow = LibraryScanRunsQuery["libraryScanRuns"]["edges"][number]["node"];
type IssueRow = LibraryScanIssuesQuery["libraryScanIssues"]["edges"][number]["node"];

const SEVERITY: Record<string, { tone: "danger" | "warning" | "accent" | "default"; dot: string }> = {
  ERROR: { tone: "danger", dot: "bg-danger" },
  WARNING: { tone: "warning", dot: "bg-warning" },
  INFO: { tone: "accent", dot: "bg-info" },
};

/** Scan history and unresolved issues for one library. */
export function ScansPanel({ libraryId }: { libraryId: string }) {
  const isAdmin = useIsAdmin();
  const scan = useLibraryScan(libraryId);
  const [showResolved, setShowResolved] = useQueryState("resolved", parseAsBoolean.withDefault(false).withOptions({ history: "push" }));
  const [query, setQuery] = useQueryState("q", parseAsString.withDefault(""));
  const [severity, setSeverity] = useQueryState("severity", parseAsStringEnum<string>(["all", "ERROR", "WARNING", "INFO"]).withDefault("all").withOptions({ history: "push" }));
  const [runStatus, setRunStatus] = useQueryState("status", parseAsStringEnum<string>(["all", "QUEUED", "RUNNING", "COMPLETED", "COMPLETED_WITH_ISSUES", "FAILED", "CANCELLED"]).withDefault("all").withOptions({ history: "push" }));
  const search = useDebouncedValue(query.trim(), 250);
  const issueTable = useServerTable<LibraryScanIssueOrderByInput>({ persistKey: `scan-issues.${libraryId}`, defaultSorting: [{ id: "updatedAt", desc: true }] });
  const runTable = useServerTable<LibraryScanRunOrderByInput>({ persistKey: `scan-runs.${libraryId}`, defaultSorting: [{ id: "createdAt", desc: true }] });
  const issueWhere = useMemo<LibraryScanIssueWhereInput>(() => ({
    libraryId: { eq: libraryId },
    ...(!showResolved ? { resolvedAt: { isNull: true } } : {}),
    ...(search ? { issueCode: { contains: search } } : {}),
    ...(severity !== "all" ? { severity: { eq: severity } } : {}),
  }), [libraryId, showResolved, search, severity]);
  const runs = useInfiniteConnection(EntityLibraryScanRunListDocument, { where: { libraryId: { eq: libraryId }, ...(runStatus !== "all" ? { status: { eq: runStatus } } : {}) }, orderBy: runTable.orderBy }, (data) => data.libraryScanRuns);
  const issues = useInfiniteConnection(EntityLibraryScanIssueListDocument, { where: issueWhere, orderBy: issueTable.orderBy }, (data) => data.libraryScanIssues);

  const [retryIssue] = useMutation(RetryScanIssueDocument);
  const [resolveIssue] = useMutation(ResolveScanIssueDocument);
  const [trashDuplicate] = useMutation(TrashDuplicateScanIssueDocument);

  const act = async (task: () => Promise<{ success: boolean; message: string }>) => {
    try {
      const result = await task();
      if (result.success) toast.success(result.message);
      else toast.warning(result.message);
      void issues.refetch();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const latest = scan.latestRun;

  const runColumns = useMemo<Array<DataTableColumn<RunRow>>>(
    () => [
      { id: "createdAt", header: "Started", sortable: true, cell: (row) => <span className="text-foreground">{formatDateTime(row.startedAt ?? row.createdAt)}</span> },
      { id: "status", header: "Status", size: 160, sortable: true, cell: (row) => <StatusChip status={scanStatus(row.status)} /> },
      { id: "discovered", header: "Found", size: 90, align: "end", numeric: true, cell: (row) => row.discoveredCount },
      { id: "matched", header: "Matched", size: 90, align: "end", numeric: true, cell: (row) => row.matchedCount },
      { id: "unmatched", header: "Unmatched", size: 100, align: "end", numeric: true, hideBelow: "md", cell: (row) => row.unmatchedCount },
      { id: "missing", header: "Missing", size: 90, align: "end", numeric: true, hideBelow: "md", cell: (row) => row.missingCount },
      { id: "summary", header: "Summary", size: 320, hideBelow: "lg", cell: (row) => <span className="text-muted">{row.summary ?? row.errorCode ?? "—"}</span> },
    ],
    [],
  );

  const issueColumns = useMemo<Array<DataTableColumn<IssueRow>>>(
    () => [
      {
        id: "message",
        header: "Issue",
        cell: (row) => (
          <span className="min-w-0">
            <span className="block break-words text-body-sm text-foreground">{row.message}</span>
            <span className="block break-words text-label-sm text-muted">{[row.issueCode, row.remediation].filter(Boolean).join(" · ")}</span>
          </span>
        ),
      },
      { id: "severity", header: "Severity", size: 120, cell: (row) => <StatusChip status={{ label: row.severity.charAt(0) + row.severity.slice(1).toLowerCase(), ...(SEVERITY[row.severity] ?? SEVERITY.INFO!) }} /> },
      { id: "stage", header: "Stage", size: 130, hideBelow: "md", cell: (row) => <span className="text-muted">{row.stage.toLowerCase().replace(/_/g, " ")}</span> },
      { id: "count", header: "Seen", size: 80, align: "end", numeric: true, hideBelow: "lg", cell: (row) => row.occurrenceCount },
      { id: "updatedAt", header: "Last", size: 130, sortable: true, hideBelow: "md", cell: (row) => <span className="text-muted">{formatRelative(row.updatedAt)}</span> },
    ],
    [],
  );

  const issueActions = useMemo<Array<DataTableRowAction<IssueRow>>>(
    () =>
      isAdmin
        ? [
            { key: "retry", label: "Retry", icon: <IconRefresh size={16} />, hidden: (row) => Boolean(row.resolvedAt) || row.stage !== "ANALYSIS", onAction: (row) => act(async () => (await retryIssue({ variables: { issueId: row.id } })).data!.retryScanIssue) },
            { key: "resolve", label: "Mark resolved", icon: <IconCheck size={16} />, hidden: (row) => Boolean(row.resolvedAt), onAction: (row) => act(async () => (await resolveIssue({ variables: { issueId: row.id, resolution: "reviewed" } })).data!.resolveScanIssue) },
            {
              key: "trash",
              label: "Move duplicate to trash",
              icon: <IconTrash size={16} />,
              destructive: true,
              hidden: (row) => Boolean(row.resolvedAt) || row.issueCode !== "BYTE_IDENTICAL_DUPLICATE",
              onAction: (row) => act(async () => (await trashDuplicate({ variables: { issueId: row.id } })).data!.trashDuplicateScanIssue),
            },
          ]
        : [],
    [isAdmin, resolveIssue, retryIssue, trashDuplicate],
  );

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <StatTile label="Last scan" value={latest ? formatRelative(latest.startedAt ?? latest.createdAt) : "Never"} hint={latest ? scanStatus(latest.status).label : undefined} tone={latest?.status === "FAILED" ? "danger" : latest?.status === "COMPLETED_WITH_ISSUES" ? "warning" : "default"} />
        <StatTile label="Files found" value={latest?.discoveredCount ?? 0} hint={latest ? `${latest.existingCount} already known` : undefined} />
        <StatTile label="Matched" value={latest?.matchedCount ?? 0} hint={latest ? `${latest.unmatchedCount} unmatched` : undefined} tone={latest && latest.unmatchedCount > 0 ? "warning" : "success"} />
        <StatTile label="Open issues" value={scan.unresolvedIssues} tone={scan.unresolvedIssues > 0 ? "warning" : "success"} />
      </div>

      <Panel
        title="Issues"
        flush
        actions={
          <Button size="sm" variant="ghost" onPress={() => setShowResolved((value) => !value)}>
            <IconHistory size={16} /> {showResolved ? "Hide resolved" : "Show resolved"}
          </Button>
        }
      >
        <div className="flex flex-wrap items-center gap-2 px-4 pb-3">
          <SearchField aria-label="Filter issue codes" placeholder="Filter issue codes" value={query} onValueChange={(value) => void setQuery(value)} className="w-full sm:w-64" />
          <Select aria-label="Issue severity" value={severity} onValueChange={(value) => setSeverity(value ?? "all")} options={[{ value: "all", label: "All severities" }, { value: "ERROR", label: "Error" }, { value: "WARNING", label: "Warning" }, { value: "INFO", label: "Info" }]} />
        </div>
        <DataTable<IssueRow>
          className="px-4 pb-4" frame={false}
          columns={issueColumns}
          rows={issues.rows}
          getRowId={(row) => row.id}
          isLoading={issues.loading}
          refetching={issues.fetching}
          queryKey={issues.queryKey} loadRevision={issues.loadRevision}
          externalBrowserToolbar
          totalCount={issues.totalCount}
          infinite={{ hasMore: issues.hasMore, loadMore: issues.loadMore, loadingMore: issues.loadingMore }}
          sorting={issueTable.sorting}
          onSortingChange={issueTable.setSorting}
          rowActions={issueActions}
          density="compact"
          noun="issues"
          error={issues.error && issues.rows.length === 0 ? <ErrorState error={issues.error} onRetry={() => void issues.refetch()} compact /> : undefined}
          emptyState={<EmptyState compact icon={IconCheck} title={search || severity !== "all" ? "No issues match" : showResolved ? "No issues" : "No open issues"} />}
        />
      </Panel>

      <Panel title="History" flush>
        <div className="px-4 pb-3">
          <Select aria-label="Scan status" value={runStatus} onValueChange={(value) => setRunStatus(value ?? "all")} options={[{ value: "all", label: "All scans" }, ...["QUEUED", "RUNNING", "COMPLETED", "COMPLETED_WITH_ISSUES", "FAILED", "CANCELLED"].map((value) => ({ value, label: scanStatus(value).label }))]} />
        </div>
        <DataTable<RunRow>
          className="px-4 pb-4" frame={false}
          columns={runColumns}
          rows={runs.rows}
          getRowId={(row) => row.id}
          isLoading={runs.loading}
          refetching={runs.fetching}
          queryKey={runs.queryKey} loadRevision={runs.loadRevision}
          externalBrowserToolbar
          totalCount={runs.totalCount}
          infinite={{ hasMore: runs.hasMore, loadMore: runs.loadMore, loadingMore: runs.loadingMore }}
          sorting={runTable.sorting}
          onSortingChange={runTable.setSorting}
          density="compact"
          noun="scans"
          error={runs.error && runs.rows.length === 0 ? <ErrorState error={runs.error} onRetry={() => void runs.refetch()} compact /> : undefined}
          emptyState={<EmptyState compact icon={IconHistory} title="No scans yet" description="Run a scan to discover files in this library." />}
        />
      </Panel>
    </div>
  );
}
