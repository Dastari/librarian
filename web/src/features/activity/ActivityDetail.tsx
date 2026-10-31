import { ActionBar, type ActionSet } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { EntityIdentity } from "glass-ux/entity";
import { IconArrowRight, IconClose } from "glass-ux/icons";

import { EntityChip } from "@/components/entities/EntityChip";
import { Button, KeyValueList, Panel } from "@/components/ui";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { fileName, humanize } from "@/lib/entities/linked";
import { notificationDecision, notificationLevel, notificationSubjects, upgradeDetails } from "@/lib/entities/notification";
import { formatDateTime, formatRelative } from "@/lib/format";

import type { ActivityRow } from "./useActivity";

const NotificationGlyph = ENTITY_KINDS.notification.icon;

/** One notification in full: what happened, what it is about, and the decision it asks for. */
export function ActivityDetail({ row, actions, onClose }: { row: ActivityRow; actions: ActionSet<ActivityRow>; onClose: () => void }) {
  const level = notificationLevel(row.notificationType);
  const decision = notificationDecision(row);
  const subjects = notificationSubjects(row);
  const upgrade = row.actionType === "quality_upgrade" ? upgradeDetails(row.actionData) : null;

  return (
    <Panel className="self-start xl:sticky xl:top-0">
      <div className="flex items-start gap-2">
        <EntityIdentity
          className="min-w-0 flex-1"
          tier="compact"
          heading="h2"
          kind={ENTITY_KINDS.notification.noun}
          name={row.title}
          icon={<NotificationGlyph />}
          facts={[level.label, humanize(row.category), formatRelative(row.createdAt)]}
        />
        <Button variant="ghost" size="sm" isIconOnly aria-label="Close notification details" onPress={onClose}><IconClose /></Button>
      </div>
      <ActionBar className="mt-3 justify-start" set={actions} targets={[row]} label={row.title} maxButtons={3} />
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Badge size="sm" marker="dot" tone={level.tone}>{level.label}</Badge>
        {decision ? <Badge size="sm" tone={decision.tone}>{decision.label}</Badge> : null}
      </div>
      <p className="mt-4 whitespace-pre-wrap break-words text-body-sm text-foreground">{row.message}</p>
      {upgrade?.from || upgrade?.to ? (
        <div className="mt-4 flex items-center gap-3 rounded-control border border-separator p-3">
          <div className="min-w-0 flex-1">
            <p className="text-overline text-muted">Current</p>
            <p className="text-title-sm">{upgrade.from ?? "Unknown"}</p>
            {upgrade.targetPath ? <p className="truncate text-label-sm text-muted" title={upgrade.targetPath}>{fileName(upgrade.targetPath)}</p> : null}
          </div>
          <IconArrowRight className="shrink-0 text-muted" />
          <div className="min-w-0 flex-1">
            <p className="text-overline text-muted">Upgrade</p>
            <p className="text-title-sm text-success">{upgrade.to ?? "Unknown"}</p>
            {upgrade.sourcePath ? <p className="truncate text-label-sm text-muted" title={upgrade.sourcePath}>{fileName(upgrade.sourcePath)}</p> : null}
          </div>
        </div>
      ) : null}
      {subjects.length ? (
        <>
          <p className="text-overline mb-2 mt-5 text-muted">About</p>
          <div className="flex flex-wrap gap-2">{subjects.map((subject) => <EntityChip key={`${subject.kind}:${subject.id}`} entity={subject} />)}</div>
        </>
      ) : null}
      <KeyValueList
        className="mt-5"
        items={[
          { label: "Received", value: formatDateTime(row.createdAt) },
          { label: "Read", value: row.readAt ? formatDateTime(row.readAt) : "Not yet" },
          { label: "Resolved", value: row.resolvedAt ? formatDateTime(row.resolvedAt) : undefined },
          { label: "Category", value: humanize(row.category) },
        ]}
      />
    </Panel>
  );
}
