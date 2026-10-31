import type { DataTableState } from "glass-ux/data-table";
import { useMemo } from "react";

import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import {
  ScanCountsDocument,
  ScanIssuesTableDocument,
  ScanRunsTableDocument,
  type LibraryScanIssueOrderByInput,
  type LibraryScanIssueWhereInput,
  type LibraryScanRunOrderByInput,
  type LibraryScanRunWhereInput,
  type ScanIssuesTableQuery,
  type ScanRunsTableQuery,
} from "@/graphql/generated/graphql";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

export type IssueRow = ScanIssuesTableQuery["libraryScanIssues"]["edges"][number]["node"];
export type RunRow = ScanRunsTableQuery["libraryScanRuns"]["edges"][number]["node"];

export const SCAN_VIEWS = ["open", "resolved", "history"] as const;
export type ScanView = (typeof SCAN_VIEWS)[number];

const BATCH = 50;

function optionValues(raw: unknown): string[] {
  if (raw && typeof raw === "object" && "values" in raw && Array.isArray(raw.values)) return raw.values.filter((value): value is string => typeof value === "string");
  return [];
}

const direction = (desc: boolean): "ASC" | "DESC" => (desc ? "DESC" : "ASC");

/** Issue table state as a server query, scoped to the library, the view and (optionally) one run. */
export function issueVariables(libraryId: string, state: DataTableState, view: "open" | "resolved", scanRunId: string | null): { where: LibraryScanIssueWhereInput; orderBy: LibraryScanIssueOrderByInput[] } {
  const conditions: LibraryScanIssueWhereInput[] = [{ libraryId: { eq: libraryId } }, { resolvedAt: { isNull: view === "open" } }];
  if (scanRunId) conditions.push({ scanRunId: { eq: scanRunId } });
  for (const { id, value } of state.filters) {
    const values = optionValues(value);
    if (!values.length) continue;
    switch (id) {
      case "severity": conditions.push({ severity: { inList: values } }); break;
      case "stage": conditions.push({ stage: { inList: values } }); break;
      case "code": conditions.push({ issueCode: { inList: values } }); break;
      default: throw new Error(`Unsupported scan issue filter: ${id}`);
    }
  }
  const query = state.search?.trim();
  if (query) conditions.push({ or: [{ message: { contains: query } }, { issueCode: { contains: query } }] });
  const orderBy = state.sorting.flatMap(({ id, desc }): LibraryScanIssueOrderByInput[] => {
    switch (id) {
      case "code": return [{ issueCode: direction(desc) }];
      case "severity": return [{ severity: direction(desc) }];
      case "stage": return [{ stage: direction(desc) }];
      case "seen": return [{ occurrenceCount: direction(desc) }];
      case "first": return [{ createdAt: direction(desc) }];
      case "last": return [{ updatedAt: direction(desc) }];
      default: return [];
    }
  });
  return { where: { and: conditions }, orderBy: orderBy.length ? orderBy : [{ updatedAt: "DESC" }] };
}

export function runVariables(libraryId: string, state: DataTableState): { where: LibraryScanRunWhereInput; orderBy: LibraryScanRunOrderByInput[] } {
  const conditions: LibraryScanRunWhereInput[] = [{ libraryId: { eq: libraryId } }];
  for (const { id, value } of state.filters) {
    const values = optionValues(value);
    if (!values.length) continue;
    if (id === "status") conditions.push({ status: { inList: values } });
    else throw new Error(`Unsupported scan run filter: ${id}`);
  }
  const query = state.search?.trim();
  if (query) conditions.push({ or: [{ errorCode: { contains: query } }, { currentStage: { contains: query } }] });
  const orderBy = state.sorting.flatMap(({ id, desc }): LibraryScanRunOrderByInput[] => {
    switch (id) {
      case "started": return [{ createdAt: direction(desc) }];
      case "status": return [{ status: direction(desc) }];
      case "found": return [{ discoveredCount: direction(desc) }];
      case "matched": return [{ matchedCount: direction(desc) }];
      case "unmatched": return [{ unmatchedCount: direction(desc) }];
      case "missing": return [{ missingCount: direction(desc) }];
      default: return [];
    }
  });
  return { where: { and: conditions }, orderBy: orderBy.length ? orderBy : [{ createdAt: "DESC" }] };
}

export function useScanIssues(libraryId: string, state: DataTableState, view: ScanView, scanRunId: string | null) {
  const variables = useMemo(() => issueVariables(libraryId, state, view === "resolved" ? "resolved" : "open", scanRunId), [libraryId, state, view, scanRunId]);
  const list = useSettingsConnection(ScanIssuesTableDocument, variables, (data) => data.libraryScanIssues, { infinite: true, batchSize: BATCH, live: true, skip: view === "history" });
  return { list, variables };
}

export function useScanRuns(libraryId: string, state: DataTableState, view: ScanView) {
  const variables = useMemo(() => runVariables(libraryId, state), [libraryId, state]);
  const list = useSettingsConnection(ScanRunsTableDocument, variables, (data) => data.libraryScanRuns, { infinite: true, batchSize: BATCH, live: true, skip: view !== "history" });
  return { list, variables };
}

/** View counts, plus the last runs' totals oldest first for the header's trend metrics. */
export function useScanCounts(libraryId: string) {
  const counts = useQuery(ScanCountsDocument, { variables: { libraryId } });
  const recent = useMemo(() => [...(counts.data?.recent.edges ?? [])].map((edge) => edge.node).reverse(), [counts.data]);
  const views: Partial<Record<ScanView, number>> = counts.data ? {
    open: counts.data.open.pageInfo.totalCount ?? undefined,
    resolved: counts.data.resolved.pageInfo.totalCount ?? undefined,
    history: counts.data.runs.pageInfo.totalCount ?? undefined,
  } : {};
  return { views, recent, refetch: () => void counts.refetch() };
}
