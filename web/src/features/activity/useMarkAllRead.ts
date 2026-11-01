import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { useCallback } from "react";

import { MarkAllNotificationsReadDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";

const items = (count: number) => `${count} ${count === 1 ? "item" : "items"}`;

/** Acknowledges the caller's whole feed, notifications and scan issues on every page. */
export function useMarkAllRead(onChanged: () => void) {
  const [mutate, { loading }] = useMutation(MarkAllNotificationsReadDocument);
  const markAllRead = useCallback(async () => {
    try {
      const result = (await mutate()).data?.markAllNotificationsRead;
      if (!result) throw new Error("Could not mark everything read");
      const total = result.notificationCount + result.scanIssueCount;
      toast.success(total ? `Marked ${items(total)} read` : "Everything was already read");
    } catch (error) {
      toast.danger(errorMessage(error, "Could not mark everything read"));
    } finally {
      onChanged();
    }
  }, [mutate, onChanged]);
  return { markAllRead, loading };
}
