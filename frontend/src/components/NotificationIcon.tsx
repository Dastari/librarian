import { Button } from "@heroui/button";
import { Badge } from "@heroui/badge";
import { IconBell } from "@tabler/icons-react";
import {
  NotificationsDocument,
  NotificationChangedDocument,
} from "../lib/graphql/generated/graphql";
import { useQuery, useSubscription } from "../lib/graphql/client";
import { NotificationPopover } from "./NotificationPopover";
import { ErrorBoundary } from "./ErrorBoundary";

const UNREAD_WHERE = { ReadAt: { isNull: true } } as const;

function useUnreadNotificationCount() {
  const { data, previousData, refetch } = useQuery(NotificationsDocument, {
    variables: {
      Where: UNREAD_WHERE,
      Page: { limit: 1, offset: 0 },
    },
    fetchPolicy: "cache-and-network",
  });

  useSubscription(NotificationChangedDocument, {
    variables: {},
    onData: () => {
      void refetch();
    },
  });

  return (
    data?.Notifications?.PageInfo?.TotalCount ??
    previousData?.Notifications?.PageInfo?.TotalCount ??
    0
  );
}

function NotificationIconInner() {
  const unreadCount = useUnreadNotificationCount();

  return (
    <NotificationPopover
      trigger={
        <Button
          isIconOnly
          variant="light"
          size="sm"
          aria-label={`${unreadCount} unread notifications`}
          title={
            unreadCount > 0
              ? `${unreadCount} unread notification${unreadCount !== 1 ? "s" : ""}`
              : "No unread notifications"
          }
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
