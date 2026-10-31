import { ActionBar, type ActionSet } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { EntityIdentity } from "glass-ux/entity";
import { IconClose } from "glass-ux/icons";

import { EntityChip } from "@/components/entities/EntityChip";
import { Button, KeyValueList, Panel } from "@/components/ui";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { fileName, linkedItem, mediaFileRef } from "@/lib/entities/linked";
import { issueDetails, issueLabel, scanSeverity, scanStage } from "@/lib/entities/scan";
import { formatBytes, formatDateTime, formatRelative } from "@/lib/format";

import type { IssueRow } from "./useScans";

const IssueGlyph = ENTITY_KINDS.scanIssue.icon;

/** Duplicate issues compare the copy the library keeps with the redundant one. */
function DuplicateComparison({ details }: { details: Record<string, string | number | boolean> }) {
  const keeper = typeof details.keeperPath === "string" ? details.keeperPath : null;
  const duplicate = typeof details.duplicatePath === "string" ? details.duplicatePath : null;
  if (!keeper || !duplicate) return null;
  const side = (title: string, path: string, quality: unknown, tone: "success" | "warning") => (
    <div className="min-w-0 flex-1 rounded-control border border-separator p-3">
      <div className="flex items-center gap-2"><p className="text-overline text-muted">{title}</p>{typeof quality === "string" ? <Badge size="sm" tone={tone}>{quality}</Badge> : null}</div>
      <p className="mt-1 break-all text-label" title={path}>{fileName(path)}</p>
      <p className="break-all font-mono text-label-sm text-muted">{path.slice(0, Math.max(0, path.length - fileName(path).length))}</p>
    </div>
  );
  return (
    <div className="mt-4 flex flex-col gap-2 sm:flex-row">
      {side("Library keeps", keeper, details.keeperQualityStatus, "success")}
      {side("Duplicate", duplicate, details.duplicateQualityStatus, "warning")}
    </div>
  );
}

/** One scan issue in full: what the scanner found, where, and what to do about it. */
export function ScanIssueDetail({ row, actions, onClose }: { row: IssueRow; actions: ActionSet<IssueRow>; onClose: () => void }) {
  const severity = scanSeverity(row.severity);
  const details = issueDetails(row.detailsJson);
  const item = row.mediaFile ? linkedItem(row.mediaFile) : null;
  const file = row.mediaFile ? mediaFileRef(row.mediaFile) : null;
  return (
    <Panel className="self-start xl:sticky xl:top-0">
      <div className="flex items-start gap-2">
        <EntityIdentity className="min-w-0 flex-1" tier="compact" heading="h2" kind={ENTITY_KINDS.scanIssue.noun} name={issueLabel(row.issueCode)} icon={<IssueGlyph />} facts={[severity.label, scanStage(row.stage), formatRelative(row.updatedAt)]} />
        <Button variant="ghost" size="sm" isIconOnly aria-label="Close issue details" onPress={onClose}><IconClose /></Button>
      </div>
      <ActionBar className="mt-3 justify-start" set={actions} targets={[row]} label={issueLabel(row.issueCode)} maxButtons={3} />
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Badge size="sm" marker="dot" tone={severity.tone}>{severity.label}</Badge>
        <Badge size="sm" tone="neutral">{scanStage(row.stage)}</Badge>
        {row.resolvedAt ? <Badge size="sm" tone="success">Resolved</Badge> : null}
      </div>
      <p className="mt-4 whitespace-pre-wrap break-words text-body-sm text-foreground">{row.message}</p>
      {row.remediation ? <p className="mt-3 rounded-control border border-separator bg-surface-secondary/40 p-3 text-body-sm text-muted">{row.remediation}</p> : null}
      {details ? <DuplicateComparison details={details} /> : null}
      {item || file ? (
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
          { label: "Issue type", value: row.issueCode, mono: true },
          { label: "Seen", value: row.occurrenceCount === 1 ? "Once" : `${row.occurrenceCount} times` },
          { label: "First seen", value: formatDateTime(row.createdAt) },
          { label: "Last seen", value: formatDateTime(row.updatedAt) },
          { label: "Resolved", value: row.resolvedAt ? `${formatDateTime(row.resolvedAt)}${row.resolution ? ` · ${row.resolution}` : ""}` : undefined },
          { label: "File size", value: row.mediaFile ? formatBytes(row.mediaFile.size) : typeof details?.size === "number" ? formatBytes(details.size) : undefined },
          { label: "Path", value: row.mediaFile?.path, mono: true },
        ]}
      />
    </Panel>
  );
}
