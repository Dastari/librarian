import { useMutation } from "@apollo/client/react";
import { ActionBar, type ActionSet } from "glass-ux/actions";
import { toast } from "glass-ux/toast";
import { useState } from "react";
import { Tooltip } from "glass-ux/tooltip";
import { Badge } from "glass-ux/badge";
import { EntityIdentity } from "glass-ux/entity";

import { EntityChip } from "@/components/entities/EntityChip";
import { Button, KeyValueList } from "@/components/ui";
import { DuplicateCopyDocument, EntityLibraryScanIssueUpdateDocument, TrashDuplicateScanIssueDocument } from "@/graphql/generated/graphql";
import { withoutAction } from "@/lib/actions/withoutAction";
import { errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { fileName, linkedItem, mediaFileRef } from "@/lib/entities/linked";
import { issueDetails, issueLabel, scanSeverity, scanStage } from "@/lib/entities/scan";
import { formatBytes, formatDateTime, formatRelative } from "@/lib/format";

import type { IssueRow } from "./useScans";

const IssueGlyph = ENTITY_KINDS.scanIssue.icon;

type CopyFile = NonNullable<IssueRow["mediaFile"]>;

/** One copy of a duplicate pair: where it is, what plays it, and the choice to keep it. */
function DuplicateCopy({ path, file, best, onKeep, busy, disabled }: { path: string; file: CopyFile | null; best: boolean; onKeep: () => void; busy: boolean; disabled: boolean }) {
  const item = file ? linkedItem(file) : null;
  const folder = path.slice(0, Math.max(0, path.length - fileName(path).length));
  return (
    <div className={`flex min-w-0 flex-1 flex-col gap-3 rounded-control border p-4 ${best ? "border-success/50 bg-success/5" : "border-separator"}`}>
      <div className="flex min-w-0 flex-col gap-1">
        <div className="flex items-center gap-2">
          <Tooltip content={path}><p className="min-w-0 break-words text-label font-medium text-foreground">{fileName(path)}</p></Tooltip>
          {best ? <Badge size="sm" tone="success">Better copy</Badge> : null}
        </div>
        <p className="break-all font-mono text-label-sm text-muted">{folder}</p>
      </div>
      <div className="min-w-0">
        {item ? <EntityChip entity={item} /> : <p className="text-label-sm text-muted">No title plays this copy</p>}
      </div>
      <div className="mt-auto">
        <Button size="sm" variant={best ? "primary" : "secondary"} isPending={busy} isDisabled={disabled} onPress={onKeep}>Keep this one</Button>
      </div>
    </div>
  );
}

/**
 * A duplicate as the one decision it asks for: the two identical files side by side, what plays
 * each, and which to keep. The other goes to the library's recoverable trash.
 */
function DuplicateDecision({ row, details, onChanged }: { row: IssueRow; details: Record<string, string | number | boolean>; onChanged: () => void }) {
  const keeperPath = typeof details.keeperPath === "string" ? details.keeperPath : null;
  const duplicatePath = typeof details.duplicatePath === "string" ? details.duplicatePath : null;
  const { data } = useQuery(DuplicateCopyDocument, { variables: { libraryId: row.libraryId, path: keeperPath ?? "" }, skip: !keeperPath });
  const keeperFile = data?.mediaFiles.edges[0]?.node ?? null;
  const [trash] = useMutation(TrashDuplicateScanIssueDocument);
  const [resolve] = useMutation(EntityLibraryScanIssueUpdateDocument);
  const [busy, setBusy] = useState<"keeper" | "duplicate" | "best" | "both" | null>(null);
  if (!keeperPath || !duplicatePath) return null;
  const played = (file: CopyFile | null) => Boolean(file && linkedItem(file));
  // The copy a title plays is the one to keep; with both or neither played, the scan's choice.
  const duplicateBest = played(row.mediaFile ?? null) && !played(keeperFile);
  const run = async (choice: "keeper" | "duplicate" | "best" | "both") => {
    setBusy(choice);
    try {
      if (choice === "both") {
        const result = (await resolve({ variables: { id: row.id, input: { resolvedAt: new Date().toISOString(), resolution: "Kept both copies" } } })).data?.updateLibraryScanIssue;
        if (!result?.success) throw new Error(result?.error ?? "Could not keep both");
        toast.success("Kept both copies");
      } else {
        const result = (await trash({ variables: { issueId: row.id, keep: choice } })).data?.trashDuplicateScanIssue;
        if (!result?.success) throw new Error(result?.message ?? "Could not move the other copy");
        toast.success("Moved the other copy to the library's trash");
      }
      onChanged();
    } catch (error) {
      toast.danger(errorMessage(error));
    } finally {
      setBusy(null);
    }
  };
  const resolved = Boolean(row.resolvedAt);
  return (
    <div className="mt-4 flex flex-col gap-4">
      <p className="text-body-sm text-foreground">These two files are identical, byte for byte. Keep one and the other moves to the library&apos;s trash, where it can be restored.</p>
      <div className="flex flex-col gap-3 sm:flex-row">
        <DuplicateCopy path={keeperPath} file={keeperFile} best={!duplicateBest} busy={busy === "keeper"} disabled={resolved || busy !== null} onKeep={() => void run("keeper")} />
        <DuplicateCopy path={duplicatePath} file={row.mediaFile ?? null} best={duplicateBest} busy={busy === "duplicate"} disabled={resolved || busy !== null} onKeep={() => void run("duplicate")} />
      </div>
      {resolved ? null : (
        <div className="flex flex-wrap items-center gap-2">
          <Button size="sm" variant="ghost" isPending={busy === "both"} isDisabled={busy !== null} onPress={() => void run("both")}>Keep both</Button>
        </div>
      )}
    </div>
  );
}

/** One scan issue in full: what the scanner found, where, and what to do about it. */
export function ScanIssueDetail({ row, actions, onChanged }: { row: IssueRow; actions: ActionSet<IssueRow>; onChanged: () => void }) {
  const severity = scanSeverity(row.severity);
  const details = issueDetails(row.detailsJson);
  const duplicate = row.issueCode === "BYTE_IDENTICAL_DUPLICATE" && details !== null;
  // A duplicate's choices are in its comparison, so the header keeps only the rest.
  const headerActions = duplicate ? withoutAction(actions, "trash-duplicate", "keep-best", "resolve") : actions;
  const item = row.mediaFile ? linkedItem(row.mediaFile) : null;
  const file = row.mediaFile ? mediaFileRef(row.mediaFile) : null;
  return (
    <div className="flex min-w-0 flex-col">
      <div className="flex items-start gap-2">
        <EntityIdentity className="min-w-0 flex-1" tier="compact" heading="h2" kind={ENTITY_KINDS.scanIssue.noun} name={issueLabel(row.issueCode)} icon={<IssueGlyph />} facts={[severity.label, scanStage(row.stage), formatRelative(row.updatedAt)]} />
        <ActionBar set={headerActions} targets={[row]} label={issueLabel(row.issueCode)} maxButtons={3} />
      </div>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Badge size="sm" marker="dot" tone={severity.tone}>{severity.label}</Badge>
        <Badge size="sm" tone="neutral">{scanStage(row.stage)}</Badge>
        {row.resolvedAt ? <Badge size="sm" tone="success">Resolved</Badge> : null}
      </div>
      {duplicate && details ? <DuplicateDecision row={row} details={details} onChanged={onChanged} /> : (
        <>
          <p className="mt-4 whitespace-pre-wrap break-words text-body-sm text-foreground">{row.message}</p>
          {row.remediation ? <p className="mt-3 rounded-control border border-separator bg-surface-secondary/40 p-3 text-body-sm text-muted">{row.remediation}</p> : null}
        </>
      )}
      {!duplicate && (item || file) ? (
        <>
          <p className="text-overline mb-2 mt-5 text-muted">About</p>
          <div className="flex flex-wrap gap-2">
            {item ? <EntityChip entity={item} /> : null}
            {file ? <EntityChip entity={file} /> : null}
          </div>
        </>
      ) : null}
      <KeyValueList
        className="mt-5"
        items={[
          { label: "Issue type", value: duplicate ? undefined : row.issueCode, mono: true },
          { label: "Seen", value: row.occurrenceCount === 1 ? "Once" : `${row.occurrenceCount} times` },
          { label: "First seen", value: formatDateTime(row.createdAt) },
          { label: "Last seen", value: formatDateTime(row.updatedAt) },
          { label: "Resolved", value: row.resolvedAt ? `${formatDateTime(row.resolvedAt)}${row.resolution ? ` · ${row.resolution}` : ""}` : undefined },
          { label: "File size", value: row.mediaFile ? formatBytes(row.mediaFile.size) : typeof details?.size === "number" ? formatBytes(details.size) : undefined },
          { label: "Path", value: duplicate ? undefined : row.mediaFile?.path, mono: true },
        ]}
      />
    </div>
  );
}
