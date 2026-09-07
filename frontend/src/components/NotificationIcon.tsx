import {
  useNotificationOwner,
  useNotificationRefresh,
} from "../hooks/useNotificationFeed";
import { Button } from "@heroui/button";
import { Badge } from "@heroui/badge";
import { IconBell } from "@tabler/icons-react";
import {
  NotificationsDocument,
  NotificationChangedDocument,
  UnresolvedLibraryScanIssuesDocument,
  LibraryScanIssueNotificationChangedDocument,
} from "../lib/graphql/generated/graphql";
import { useQuery, useSubscription } from "../lib/graphql/client";
import { NotificationPopover } from "./NotificationPopover";
import { ErrorBoundary } from "./ErrorBoundary";

const UNREAD_WHERE = { readAt: { isNull: true } } as const;

function usePendingNotificationCount() {
  const owner = useNotificationOwner();
  const notificationsQuery = useQuery(NotificationsDocument, {
    variables: {
      where: { ...owner, ...UNREAD_WHERE },
      page: { limit: 1, offset: 0 },
    },
    fetchPolicy: "cache-and-network",
  });

  const scanIssuesQuery = useQuery(UnresolvedLibraryScanIssuesDocument, {
    variables: {
      where: {
        ...owner,
        readAt: { isNull: true },
        resolvedAt: { isNull: true },
      },
      page: { limit: 1, offset: 0 },
    },
    fetchPolicy: "cache-and-network",
  });

  const refreshNotifications = useNotificationRefresh(() =>
    notificationsQuery.refetch(),
  );
  const refreshScanIssues = useNotificationRefresh(() =>
    scanIssuesQuery.refetch(),
  );
  useSubscription(NotificationChangedDocument, {
    fetchPolicy: "no-cache",
    ignoreResults: true,
    onData: refreshNotifications,
  });
  useSubscription(LibraryScanIssueNotificationChangedDocument, {
    fetchPolicy: "no-cache",
    ignoreResults: true,
    onData: refreshScanIssues,
  });

  const unreadNotificationCount =
    notificationsQuery.data?.notifications?.pageInfo?.totalCount ??
    notificationsQuery.previousData?.notifications?.pageInfo?.totalCount ??
    0;
  const unresolvedScanIssueCount =
    scanIssuesQuery.data?.libraryScanIssues.pageInfo.totalCount ??
    scanIssuesQuery.previousData?.libraryScanIssues.pageInfo.totalCount ??
    0;

  return unreadNotificationCount + unresolvedScanIssueCount;
}

function pendingNotificationLabel(count: number): string {
  return count > 0
    ? `${count} pending notification${count !== 1 ? "s" : ""}`
    : "No pending notifications";
}

function NotificationIconInner() {
  const pendingCount = usePendingNotificationCount();
  const label = pendingNotificationLabel(pendingCount);

  return (
    <NotificationPopover
      pendingCount={pendingCount}
      trigger={
        <Button
          isIconOnly
          variant="light"
          size="sm"
          aria-label={label}
          title={label}
        >
          <Badge
            content={pendingCount}
            color="warning"
            size="sm"
            isInvisible={pendingCount === 0}
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
 * Notification bell with a badge for unread notifications and scan issues.
 * Uses codegen queries and subscriptions for both durable record types.
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
