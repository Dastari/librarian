import { ActionBar, type ActionSet } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { EntityIdentity } from "glass-ux/entity";
import { IconClose, IconHistory } from "glass-ux/icons";
import { MeterCell } from "glass-ux/meter-cell";

import { Button, KeyValueList, Panel } from "@/components/ui";
import { scanDuration, scanRunStatus, scanStage } from "@/lib/entities/scan";
import { formatDateTime, formatRelative } from "@/lib/format";

import type { RunRow } from "./useScans";

/** A pipeline step as a bar: how much of what it took on succeeded. */
function Step({ label, done, failed, total }: { label: string; done: number; failed?: number; total: number }) {
  return (
    <div className="grid grid-cols-[8rem_minmax(0,1fr)] items-center gap-3">
      <span className="text-label-sm text-muted">{label}</span>
      <MeterCell label={label} value={total ? done / total : 0} max={1} tone="data" warningAt={2} dangerAt={2} valueText={`${done.toLocaleString()} of ${total.toLocaleString()}${failed ? ` · ${failed} failed` : ""}`} />
    </div>
  );
}

/** One scan run: its outcome and how far each pipeline step got. */
export function ScanRunDetail({ row, actions, onClose }: { row: RunRow; actions: ActionSet<RunRow>; onClose: () => void }) {
  const status = scanRunStatus(row.status);
  const started = row.startedAt ?? row.createdAt;
  const issues = row.issues.pageInfo.totalCount ?? 0;
  const matchTotal = row.matchedCount + row.unmatchedCount + row.providerBlockedCount;
  return (
    <Panel className="self-start xl:sticky xl:top-0">
      <div className="flex items-start gap-2">
        <EntityIdentity className="min-w-0 flex-1" tier="compact" heading="h2" kind="Scan" name={formatDateTime(started)} icon={<IconHistory />} facts={[status.label, scanDuration(row) ?? scanStage(row.currentStage), formatRelative(started)]} />
        <Button variant="ghost" size="sm" isIconOnly aria-label="Close scan details" onPress={onClose}><IconClose /></Button>
      </div>
      <ActionBar className="mt-3 justify-start" set={actions} targets={[row]} label={`Scan ${formatDateTime(started)}`} maxButtons={2} />
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Badge size="sm" marker="dot" tone={status.tone}>{status.label}</Badge>
        {issues ? <Badge size="sm" tone="warning">{issues === 1 ? "1 issue" : `${issues} issues`}</Badge> : null}
      </div>
      {row.summary ? <p className="mt-4 text-body-sm text-foreground">{row.summary}</p> : null}
      <p className="text-overline mb-3 mt-5 text-muted">Pipeline</p>
      <div className="flex flex-col gap-2.5">
        <Step label="Already known" done={row.existingCount} total={row.discoveredCount} />
        <Step label="Matched" done={row.matchedCount} failed={row.providerBlockedCount} total={matchTotal} />
        <Step label="Analysed" done={row.analysisSucceededCount} failed={row.analysisFailedCount} total={row.analysisQueuedCount || row.analysisSucceededCount + row.analysisFailedCount} />
        <Step label="Organised" done={row.organizationSucceededCount} failed={row.organizationFailedCount} total={row.organizationSucceededCount + row.organizationFailedCount} />
        <Step label="Reconciled" done={row.reconciledCount} total={row.reconciledCount + row.missingCount} />
      </div>
      <KeyValueList
        className="mt-5"
        items={[
          { label: "Files found", value: row.discoveredCount.toLocaleString() },
          { label: "Unmatched", value: row.unmatchedCount.toLocaleString() },
          { label: "Provider blocked", value: row.providerBlockedCount ? row.providerBlockedCount.toLocaleString() : undefined },
          { label: "Missing", value: row.missingCount.toLocaleString() },
          { label: "Finished", value: row.finishedAt ? formatDateTime(row.finishedAt) : undefined },
          { label: "Error", value: row.errorCode ? <span className="font-mono text-danger">{row.errorCode}</span> : undefined },
        ]}
      />
    </Panel>
  );
}
