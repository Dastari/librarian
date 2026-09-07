import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { addToast } from "@heroui/toast";
import {
  apolloClient,
  onWebSocketConnectionState,
} from "../lib/graphql/client";
import {
  MarkAllNotificationsReadDocument,
  NotificationsDocument,
  UnresolvedLibraryScanIssuesDocument,
} from "../lib/graphql/generated/graphql";
import { sanitizeError } from "../lib/format";
import { useAuth } from "./useAuth";

export function useNotificationOwner() {
  const { user } = useAuth();
  return useMemo(() => ({ userId: { eq: user?.id ?? "" } }), [user?.id]);
}

/** Coalesce a transaction's row events; never overlap refreshes or starve a busy feed. */
export function useNotificationRefresh(refresh: () => Promise<unknown>) {
  const callback = useRef(refresh);
  callback.current = refresh;
  const schedule = useRef<() => void>(() => {});
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    let running = false;
    let dirty = false;
    let disposed = false;
    const enqueue = () => {
      dirty = true;
      if (disposed || running || timer) return;
      timer = setTimeout(async () => {
        timer = undefined;
        running = true;
        dirty = false;
        try {
          await callback.current();
        } catch {
          /* Queries retain their error state for retry. */
        } finally {
          running = false;
          if (dirty && !disposed) enqueue();
        }
      }, 300);
    };
    schedule.current = enqueue;
    let previousConnection: string | undefined;
    const unsubscribeConnection = onWebSocketConnectionState((state) => {
      const previous = previousConnection;
      previousConnection = state.status;
      // Events can be missed during session renewal or a network interruption.
      // Refresh once when the subscription transport reconnects.
      if (
        previous !== undefined &&
        previous !== "connected" &&
        state.status === "connected"
      )
        enqueue();
    });
    return () => {
      disposed = true;
      unsubscribeConnection();
      clearTimeout(timer);
      schedule.current = () => {};
    };
  }, []);
  return useCallback(() => schedule.current(), []);
}

let pendingMarkAll: Promise<number> | undefined;
export function markAllNotificationsRead(): Promise<number> {
  if (pendingMarkAll) return pendingMarkAll;
  pendingMarkAll = (async () => {
    const result = await apolloClient.mutate({
      mutation: MarkAllNotificationsReadDocument,
    });
    const counts = result.data?.markAllNotificationsRead;
    if (!counts)
      throw new Error("The server did not acknowledge the notifications");
    await apolloClient.refetchQueries({
      include: [NotificationsDocument, UnresolvedLibraryScanIssuesDocument],
    });
    return counts.notificationCount + counts.scanIssueCount;
  })().finally(() => {
    pendingMarkAll = undefined;
  });
  return pendingMarkAll;
}

export function useMarkAllNotificationsRead() {
  const [markingAllRead, setMarkingAllRead] = useState(false);
  const handleMarkAllRead = useCallback(async () => {
    setMarkingAllRead(true);
    try {
      const count = await markAllNotificationsRead();
      addToast({
        title: "Marked as read",
        description: `${count} notifications and scan issues acknowledged`,
        color: "success",
      });
    } catch (error) {
      addToast({
        title: "Could not mark all as read",
        description: sanitizeError(error),
        color: "danger",
      });
    } finally {
      setMarkingAllRead(false);
    }
  }, []);
  return { handleMarkAllRead, markingAllRead };
}
