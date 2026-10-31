import { useMutation } from "@apollo/client/react";
import { defineActions, type ActionDefinition, type ActionSet } from "glass-ux/actions";
import { IconCheck, IconExternalLink, IconFile, IconRefresh, IconTrash } from "glass-ux/icons";
import { useMemo } from "react";

import { ResolveScanIssueDocument, RetryScanIssueDocument, TrashDuplicateScanIssueDocument } from "@/graphql/generated/graphql";
import { ENTITY_KINDS, type EntityKindName } from "@/lib/entities/kinds";
import { linkedItem, mediaFileRef } from "@/lib/entities/linked";

import type { IssueRow } from "./useScans";

const plural = (rows: readonly IssueRow[], one: string, many: string) => (rows.length === 1 ? one : many.replace("{n}", String(rows.length)));
const open = (row: IssueRow) => !row.resolvedAt;

/** Throws the server's message when a pipeline action reports it did not happen. */
function check(result: { success: boolean; message: string } | undefined, fallback: string) {
  if (!result?.success) throw new Error(result?.message || fallback);
}

/**
 * A scan issue's actions for its row menu, right-click, the bulk bar and the detail pane.
 * Retry re-runs analysis; trashing a duplicate moves the redundant copy to the trash.
 */
export function useScanIssueActions(onChanged: () => void, isAdmin: boolean): ActionSet<IssueRow> {
  const [retry] = useMutation(RetryScanIssueDocument);
  const [resolve] = useMutation(ResolveScanIssueDocument);
  const [trash] = useMutation(TrashDuplicateScanIssueDocument);

  return useMemo(() => {
    const item = (row: IssueRow) => (row.mediaFile ? linkedItem(row.mediaFile) : null);
    const actions: ActionDefinition<IssueRow>[] = [
      {
        id: "open-item", label: (rows) => `Open ${ENTITY_KINDS[item(rows[0]!)?.kind as EntityKindName]?.noun.toLowerCase() ?? "item"}`, icon: IconExternalLink, group: "open", priority: 2,
        visible: (rows) => rows.length === 1 && Boolean(item(rows[0]!)?.href),
        run: (rows, context) => { const href = item(rows[0]!)?.href; if (href) context.navigate(href); },
      },
      {
        id: "open-file", label: "Show file", icon: IconFile, group: "open", priority: 3,
        visible: (rows) => rows.length === 1 && Boolean(rows[0]!.mediaFile),
        run: (rows, context) => { const href = rows[0]!.mediaFile ? mediaFileRef(rows[0]!.mediaFile).href : undefined; if (href) context.navigate(href); },
      },
    ];
    if (isAdmin) {
      actions.push(
        {
          id: "retry", label: (rows) => plural(rows, "Retry analysis", "Retry {n}"), icon: IconRefresh, group: "manage", priority: 1, bulk: true,
          visible: (rows) => rows.every((row) => open(row) && row.stage === "ANALYSIS"),
          run: async (rows) => { for (const row of rows) check((await retry({ variables: { issueId: row.id } })).data?.retryScanIssue, "Retry failed"); onChanged(); },
          successMessage: (rows) => plural(rows, "Analysis retried", "Retried {n} files"),
        },
        {
          id: "trash-duplicate", label: (rows) => plural(rows, "Move duplicate to trash", "Move {n} duplicates to trash"), icon: IconTrash, group: "danger", priority: 1, bulk: true,
          visible: (rows) => rows.every((row) => open(row) && row.issueCode === "BYTE_IDENTICAL_DUPLICATE"),
          confirm: (rows) => ({ title: plural(rows, "Move the duplicate copy to the trash?", "Move {n} duplicate copies to the trash?"), body: "Only the duplicate moves, into the library's .librarian-trash folder where it can be restored. The copy the library uses stays where it is.", confirmLabel: "Move to trash", tone: "warning" }),
          run: async (rows) => { for (const row of rows) check((await trash({ variables: { issueId: row.id } })).data?.trashDuplicateScanIssue, "Could not move the duplicate"); onChanged(); },
          successMessage: (rows) => plural(rows, "Duplicate moved to trash", "Moved {n} duplicates to trash"),
        },
        {
          id: "resolve", label: (rows) => plural(rows, "Mark resolved", "Mark {n} resolved"), icon: IconCheck, group: "manage", priority: 2, bulk: true,
          visible: (rows) => rows.some(open),
          run: async (rows) => { for (const row of rows.filter(open)) check((await resolve({ variables: { issueId: row.id, resolution: "reviewed" } })).data?.resolveScanIssue, "Could not resolve"); onChanged(); },
          successMessage: (rows) => plural(rows, "Resolved", "Resolved {n} issues"),
        },
      );
    }
    return defineActions("scan-issue", actions);
  }, [isAdmin, onChanged, resolve, retry, trash]);
}
