import { useMutation } from "@apollo/client/react";
import { defineActions, type ActionDefinition, type ActionSet } from "glass-ux/actions";
import { IconCheck, IconExternalLink, IconTrash, IconX } from "glass-ux/icons";
import { IconArrowUpCircle } from "@tabler/icons-react";
import { useMemo } from "react";

import { ApproveQualityUpgradeDocument, EntityNotificationDeleteManyDocument, EntityNotificationUpdateDocument, EntityNotificationUpdateManyDocument } from "@/graphql/generated/graphql";
import { ENTITY_KINDS, glyph, type EntityKindName } from "@/lib/entities/kinds";
import { notificationSubjects } from "@/lib/entities/notification";
import { assertSuccess } from "@/lib/graphql/errors";

import type { ActivityRow } from "./useActivity";

const UpgradeIcon = glyph(IconArrowUpCircle, "arrow-up-circle");
const plural = (rows: readonly ActivityRow[], one: string, many: string) => (rows.length === 1 ? one : many.replace("{n}", String(rows.length)));
const now = () => new Date().toISOString();
const pendingUpgrade = (row: ActivityRow) => row.actionType === "quality_upgrade" && !row.resolvedAt;

/**
 * A notification's actions, defined once for its row menu, right-click, the bulk bar and the
 * detail pane (docs/entity-tables.md).
 */
export function useNotificationActions(onChanged: () => void, isAdmin: boolean): ActionSet<ActivityRow> {
  const [update] = useMutation(EntityNotificationUpdateDocument);
  const [updateMany] = useMutation(EntityNotificationUpdateManyDocument);
  const [removeMany] = useMutation(EntityNotificationDeleteManyDocument);
  const [approve] = useMutation(ApproveQualityUpgradeDocument);

  return useMemo(() => {
    const ids = (rows: readonly ActivityRow[]) => ({ id: { inList: rows.map((row) => row.id) } });
    const actions: ActionDefinition<ActivityRow>[] = [
      {
        id: "open-subject", label: (rows) => { const subject = notificationSubjects(rows[0]!)[0]; return `Open ${(subject ? ENTITY_KINDS[subject.kind as EntityKindName]?.noun : undefined)?.toLowerCase() ?? "item"}`; }, icon: IconExternalLink, group: "open", priority: 2,
        visible: (rows) => rows.length === 1 && Boolean(notificationSubjects(rows[0]!)[0]?.href),
        run: (rows, context) => { const href = notificationSubjects(rows[0]!)[0]?.href; if (href) context.navigate(href); },
      },
      {
        id: "read", label: (rows) => plural(rows, "Mark read", "Mark {n} read"), icon: IconCheck, group: "manage", priority: 1, bulk: true,
        visible: (rows) => rows.some((row) => !row.readAt),
        run: async (rows) => { await updateMany({ variables: { where: ids(rows.filter((row) => !row.readAt)), input: { readAt: now() } } }); onChanged(); },
      },
      {
        id: "approve", label: (rows) => plural(rows, "Approve upgrade", "Approve {n} upgrades"), icon: UpgradeIcon, group: "manage", priority: 1, bulk: true,
        visible: (rows) => isAdmin && rows.length > 0 && rows.every(pendingUpgrade),
        confirm: (rows) => ({ title: plural(rows, "Replace the file with the upgrade?", "Replace {n} files with their upgrades?"), body: "A copy of the download replaces the current library file, which is overwritten. The download stays where it is, so seeding continues. This cannot be undone.", confirmLabel: "Replace file", tone: "warning" }),
        run: async (rows) => {
          for (const row of rows) assertSuccess((await approve({ variables: { notificationId: row.id } })).data?.approveQualityUpgrade, "Could not approve the upgrade");
          onChanged();
        },
        successMessage: (rows) => plural(rows, "Upgrade approved", "{n} upgrades approved"),
      },
      {
        id: "dismiss", label: (rows) => plural(rows, "Dismiss", "Dismiss {n}"), icon: IconX, group: "manage", priority: 2, bulk: true,
        visible: (rows) => rows.length > 0 && rows.every((row) => row.actionType && !row.resolvedAt),
        run: async (rows) => {
          for (const row of rows) await update({ variables: { id: row.id, input: { resolvedAt: now(), readAt: row.readAt ?? now(), resolution: "dismissed" } } });
          onChanged();
        },
        successMessage: (rows) => plural(rows, "Dismissed", "Dismissed {n}"),
      },
      {
        id: "delete", label: (rows) => plural(rows, "Delete", "Delete {n}"), icon: IconTrash, group: "danger", bulk: true,
        confirm: (rows) => (rows.length > 1 ? { title: `Delete ${rows.length} notifications?`, body: "Pending decisions among them are dropped too.", confirmLabel: "Delete", tone: "danger" } : null),
        run: async (rows) => { await removeMany({ variables: { where: ids(rows) } }); onChanged(); },
        successMessage: (rows) => plural(rows, "Deleted", "Deleted {n} notifications"),
      },
    ];
    return defineActions("notification", actions);
  }, [approve, isAdmin, onChanged, removeMany, update, updateMany]);
}
