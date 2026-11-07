import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { useState } from "react";

import { Button, Dialog } from "@/components/ui";
import { ApproveQualityUpgradeDocument, EntityNotificationUpdateDocument } from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { notificationSubjects, upgradeDetails } from "@/lib/entities/notification";

import { UpgradeComparison } from "./ActivityDetail";
import type { ActivityRow } from "./useActivity";

/** The one decision an upgrade asks for: which file to keep. */
export function UpgradeDialog({ row, onClose, onChanged }: { row: ActivityRow; onClose: () => void; onChanged: () => void }) {
  const [approve] = useMutation(ApproveQualityUpgradeDocument);
  const [update] = useMutation(EntityNotificationUpdateDocument);
  const [busy, setBusy] = useState<"approve" | "keep" | null>(null);
  const upgrade = upgradeDetails(row.actionData);
  const subject = notificationSubjects(row)[0]?.label;

  const run = async (kind: "approve" | "keep") => {
    setBusy(kind);
    try {
      if (kind === "approve") {
        assertSuccess((await approve({ variables: { notificationId: row.id } })).data?.approveQualityUpgrade, "Could not approve the upgrade");
        toast.success("Upgrade approved");
      } else {
        assertSuccess((await update({ variables: { id: row.id, input: { resolvedAt: new Date().toISOString() } } })).data?.updateNotification, "Could not keep the current file");
      }
      onChanged();
      onClose();
    } catch (error) {
      toast.danger(errorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Dialog
      isOpen
      onOpenChange={(open) => { if (!open) onClose(); }}
      title="Replace with the upgrade?"
      description={subject}
      size="md"
      footer={(
        <>
          <Button variant="ghost" isDisabled={busy !== null} isPending={busy === "keep"} onPress={() => void run("keep")}>Keep current file</Button>
          <Button variant="primary" isDisabled={busy !== null} isPending={busy === "approve"} onPress={() => void run("approve")}>Replace file</Button>
        </>
      )}
    >
      <div className="flex flex-col gap-4">
        {upgrade ? <UpgradeComparison upgrade={upgrade} /> : null}
        <p className="text-body-sm text-muted">The current file is overwritten and can't be recovered. The download stays where it is, so seeding continues.</p>
      </div>
    </Dialog>
  );
}
