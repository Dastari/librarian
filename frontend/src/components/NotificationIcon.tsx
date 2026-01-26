import { useState, useEffect, useCallback } from "react";
import { Button } from "@heroui/button";
import { Badge } from "@heroui/badge";
import { Tooltip } from "@heroui/tooltip";
import { IconBell } from "@tabler/icons-react";
import { graphqlClient } from "../lib/graphql";
import {
  NotificationsDocument,
  NotificationChangedDocument,
} from "../lib/graphql/generated/graphql";
import { NotificationPopover } from "./NotificationPopover";
import { ErrorBoundary } from "./ErrorBoundary";

const UNREAD_WHERE = { ReadAt: { IsNull: true } } as const;

function useUnreadNotificationCount() {
  const [count, setCount] = useState(0);

  const fetchCount = useCallback(async () => {
    try {
      const { data } = await graphqlClient
        .query(NotificationsDocument, {
          Where: UNREAD_WHERE,
          Page: { Limit: 1, Offset: 0 },
        })
        .toPromise();
      const total = data?.Notifications?.PageInfo?.TotalCount;
      setCount(total ?? 0);
    } catch {
      setCount(0);
    }
  }, []);

  useEffect(() => {
    fetchCount();
  }, [fetchCount]);

  useEffect(() => {
    let sub: { unsubscribe: () => void } | null = null;
    try {
      sub = graphqlClient
        .subscription(NotificationChangedDocument, {})
        .subscribe({
          next: () => fetchCount(),
          error: () => {},
        });
    } catch {
      // subscription setup failed (e.g. client not ready)
    }
    return () => sub?.unsubscribe?.();
  }, [fetchCount]);

  return count;
}

function NotificationIconInner() {
  const unreadCount = useUnreadNotificationCount();

  return (
    <NotificationPopover
      trigger={
        <Tooltip
          content={
            unreadCount > 0
              ? `${unreadCount} unread notification${unreadCount !== 1 ? "s" : ""}`
              : "No unread notifications"
          }
        >
          <Button
            isIconOnly
            variant="light"
            size="sm"
            aria-label={`${unreadCount} unread notifications`}
          >
            <Badge
              content={unreadCount}
              color="warning"
              size="sm"
              isInvisible={unreadCount === 0}
              showOutline={false}
            >
              <IconBell size={20} className="text-amber-400" />
            </Badge>
          </Button>
        </Tooltip>
      }
    />
  );
}

/** Fallback when NotificationIcon fails (e.g. GraphQL not ready) */
function NotificationIconFallback() {
  return (
    <Button
      isIconOnly
      variant="light"
      size="sm"
      aria-label="Notifications"
      onPress={() => window.location.assign("/notifications")}
    >
      <IconBell size={20} className="text-amber-400" />
    </Button>
  );
}

/**
 * Notification bell with unread badge and popover.
 * Uses codegen Notifications query + NotificationChanged subscription.
 * Only render when the user is authenticated.
 * Wrapped in ErrorBoundary so a failure here does not take down the Navbar.
 */
export function NotificationIcon() {
  return (
    <ErrorBoundary fallback={<NotificationIconFallback />}>
      <NotificationIconInner />
    </ErrorBoundary>
  );
}
