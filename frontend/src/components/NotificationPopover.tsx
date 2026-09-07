import {
  useNotificationOwner,
  useNotificationRefresh,
  useMarkAllNotificationsRead,
} from "../hooks/useNotificationFeed";
import { parseTimestamp } from "../lib/format";
import { useMemo, useState, type ReactNode } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Popover, PopoverTrigger, PopoverContent } from "@heroui/popover";
import { Button } from "@heroui/button";
import { Divider } from "@heroui/divider";
import { Chip } from "@heroui/chip";
import { ScrollShadow } from "@heroui/scroll-shadow";
import {
  IconCheck,
  IconAlertTriangle,
  IconInfoCircle,
  IconAlertCircle,
  IconBellRinging,
} from "@tabler/icons-react";
import {
  NotificationsDocument,
  NotificationChangedDocument,
  UnresolvedLibraryScanIssuesDocument,
  LibraryScanIssueNotificationChangedDocument,
  UpdateNotificationDocument,
  DeleteNotificationDocument,
  OrderDirection,
  type NotificationsQuery,
  type UpdateNotificationMutation,
  type UpdateNotificationMutationVariables,
  type DeleteNotificationMutation,
  type DeleteNotificationMutationVariables,
} from "../lib/graphql/generated/graphql";
import {
  APPROVE_QUALITY_UPGRADE_MUTATION,
  type ApproveQualityUpgradeMutation,
  type ApproveQualityUpgradeMutationVariables,
} from "../lib/graphql/qualityProfiles";
import { useMutation, useQuery, useSubscription } from "../lib/graphql/client";
import { addToast } from "@heroui/toast";
import { sanitizeError } from "../lib/format";
import { NotificationDetailModal } from "./NotificationDetailModal";
import {
  ScanIssueDetailModal,
  type ScanIssueNotification,
} from "./ScanIssueDetailModal";

interface NotificationPopoverProps {
  trigger: ReactNode;
  pendingCount: number;
}

type NotificationNode = NotificationsQuery["notifications"]["edges"][0]["node"];
type NotificationType = "INFO" | "WARNING" | "ERROR" | "ACTION_REQUIRED";
type NotificationResolution =
  "ACCEPTED" | "REJECTED" | "DISMISSED" | "AUTO_RESOLVED";
interface NotificationItem {
  id: string;
  title: string;
  message: string;
  notificationType: NotificationType;
  category:
    | "MATCHING"
    | "PROCESSING"
    | "QUALITY"
    | "STORAGE"
    | "EXTRACTION"
    | "CONFIGURATION";
  libraryId: string | null;
  torrentId: string | null;
  mediaFileId: string | null;
  pendingMatchId: string | null;
  actionType: string | null;
  actionData: Record<string, unknown> | null;
  readAt: string | null;
  resolvedAt: string | null;
  resolution: NotificationResolution | null;
  createdAt: string;
}

function nodeToNotification(node: NotificationNode): NotificationItem {
  let actionData: Record<string, unknown> | null = null;
  if (node.actionData) {
    try {
      actionData = JSON.parse(node.actionData) as Record<string, unknown>;
    } catch {
      actionData = null;
    }
  }
  return {
    id: node.id,
    title: node.title,
    message: node.message,
    notificationType: node.notificationType as NotificationType,
    category: node.category as NotificationItem["category"],
    libraryId: node.libraryId ?? null,
    torrentId: node.torrentId ?? null,
    mediaFileId: node.mediaFileId ?? null,
    pendingMatchId: node.pendingMatchId ?? null,
    actionType: node.actionType ?? null,
    actionData,
    readAt: node.readAt ?? null,
    resolvedAt: node.resolvedAt ?? null,
    resolution: (node.resolution as NotificationResolution) ?? null,
    createdAt: node.createdAt,
  };
}

const UNREAD_WHERE = { readAt: { isNull: true } } as const;
const RECENT_ORDER: Array<{ createdAt: "ASC" | "DESC" }> = [
  { createdAt: OrderDirection.DESC },
];
const RECENT_PAGE = { limit: 10, offset: 0 } as const;

const getNotificationIcon = (type: NotificationType) => {
  switch (type) {
    case "ERROR":
      return <IconAlertCircle size={16} className="text-red-400" />;
    case "WARNING":
      return <IconAlertTriangle size={16} className="text-amber-400" />;
    case "ACTION_REQUIRED":
      return <IconBellRinging size={16} className="text-purple-400" />;
    default:
      return <IconInfoCircle size={16} className="text-blue-400" />;
  }
};

const formatTimeAgo = (dateString: string): string => {
  const date = parseTimestamp(dateString);
  if (!date) return "Unknown date";
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString();
};

function formatScanIssueCode(value: string): string {
  return value
    .toLowerCase()
    .split("_")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function NotificationPopover({
  trigger,
  pendingCount,
}: NotificationPopoverProps) {
  const owner = useNotificationOwner();
  const navigate = useNavigate();
  const { handleMarkAllRead, markingAllRead } = useMarkAllNotificationsRead();
  const [selectedNotification, setSelectedNotification] =
    useState<NotificationItem | null>(null);
  const [isDetailOpen, setIsDetailOpen] = useState(false);
  const [selectedScanIssue, setSelectedScanIssue] =
    useState<ScanIssueNotification | null>(null);
  const [updateNotification] = useMutation<
    UpdateNotificationMutation,
    UpdateNotificationMutationVariables
  >(UpdateNotificationDocument);
  const [deleteNotification] = useMutation<
    DeleteNotificationMutation,
    DeleteNotificationMutationVariables
  >(DeleteNotificationDocument);
  const [approveQualityUpgrade] = useMutation<
    ApproveQualityUpgradeMutation,
    ApproveQualityUpgradeMutationVariables
  >(APPROVE_QUALITY_UPGRADE_MUTATION);

  const notificationsQuery = useQuery(NotificationsDocument, {
    variables: {
      where: { ...owner, ...UNREAD_WHERE },
      orderBy: RECENT_ORDER,
      page: RECENT_PAGE,
    },
    fetchPolicy: "cache-and-network",
  });
  const scanIssuesQuery = useQuery(UnresolvedLibraryScanIssuesDocument, {
    variables: {
      where: { ...owner, resolvedAt: { isNull: true } },
      page: RECENT_PAGE,
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

  const notifications = useMemo(() => {
    const edges =
      notificationsQuery.data?.notifications?.edges ??
      notificationsQuery.previousData?.notifications?.edges ??
      [];

    return edges
      .map((edge) => edge?.node)
      .filter((node): node is NotificationNode => Boolean(node))
      .map((node) => nodeToNotification(node));
  }, [notificationsQuery.data, notificationsQuery.previousData]);
  const scanIssues = useMemo(
    () =>
      (
        scanIssuesQuery.data?.libraryScanIssues.edges ??
        scanIssuesQuery.previousData?.libraryScanIssues.edges ??
        []
      ).map(({ node }) => node),
    [scanIssuesQuery.data, scanIssuesQuery.previousData],
  );
  const isLoading =
    (notificationsQuery.loading &&
      !notificationsQuery.data &&
      !notificationsQuery.previousData) ||
    (scanIssuesQuery.loading &&
      !scanIssuesQuery.data &&
      !scanIssuesQuery.previousData);

  const handleMarkRead = async (id: string) => {
    await updateNotification({
      variables: { id: id, input: { readAt: new Date().toISOString() } },
    });
    void notificationsQuery.refetch();
  };

  const handleResolve = async (
    id: string,
    resolution: NotificationResolution,
  ) => {
    const now = new Date().toISOString();
    await updateNotification({
      variables: {
        id: id,
        input: {
          resolvedAt: now,
          resolution: resolution,
          readAt: now,
        },
      },
    });
    void notificationsQuery.refetch();
  };

  const handleDelete = async (id: string) => {
    await deleteNotification({ variables: { id: id } });
    void notificationsQuery.refetch();
  };

  const handleApproveQualityUpgrade = async (id: string) => {
    try {
      const result = await approveQualityUpgrade({
        variables: { notificationId: id },
      });
      if (!result.data?.approveQualityUpgrade.success) {
        throw new Error(
          result.data?.approveQualityUpgrade.error ??
            "Failed to approve upgrade",
        );
      }
      addToast({
        title: "Upgrade approved",
        description: "File replaced; re-analyzing.",
        color: "success",
      });
    } catch (error) {
      addToast({
        title: "Error",
        description: sanitizeError(error),
        color: "danger",
      });
    }
    void notificationsQuery.refetch();
  };

  const handleNotificationClick = (notification: NotificationItem) => {
    if (!notification.readAt) {
      void handleMarkRead(notification.id);
    }
    setSelectedNotification(notification);
    setIsDetailOpen(true);
  };

  const handleViewAll = () => {
    navigate({ to: "/notifications" });
  };

  const unreadCount = pendingCount;

  return (
    <>
      <Popover placement="bottom-end" offset={10}>
        <PopoverTrigger>{trigger}</PopoverTrigger>
        <PopoverContent className="w-[28rem] max-w-[calc(100vw-2rem)] p-0">
          <div className="flex flex-col">
            <div className="flex items-center justify-between px-4 py-3 border-b border-divider">
              <h3 className="text-sm font-semibold">Notifications</h3>
              {unreadCount > 0 && (
                <Button
                  size="sm"
                  variant="light"
                  color="primary"
                  onPress={handleMarkAllRead}
                  isLoading={markingAllRead}
                >
                  Mark all read
                </Button>
              )}
            </div>

            <ScrollShadow className="max-h-96">
              {isLoading ? (
                <div className="flex items-center justify-center py-8 text-default-400">
                  Loading...
                </div>
              ) : notifications.length === 0 && scanIssues.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-8 text-default-400">
                  <IconCheck size={32} className="mb-2" />
                  <span className="text-sm">No notifications</span>
                </div>
              ) : (
                <div>
                  {scanIssues.length > 0 ? (
                    <div>
                      <div className="border-b border-divider bg-default-100/50 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-default-500">
                        Scan issues (unresolved)
                      </div>
                      <div className="divide-y divide-divider">
                        {scanIssues.map((scanIssue) => (
                          <Button
                            key={scanIssue.id}
                            variant="light"
                            className="h-auto w-full justify-start rounded-none px-4 py-3 text-left"
                            onPress={() => setSelectedScanIssue(scanIssue)}
                          >
                            <div className="flex min-w-0 flex-1 gap-3">
                              <div className="mt-0.5 shrink-0">
                                {scanIssue.severity === "ERROR" ? (
                                  <IconAlertCircle
                                    size={16}
                                    className="text-red-400"
                                  />
                                ) : (
                                  <IconAlertTriangle
                                    size={16}
                                    className="text-amber-400"
                                  />
                                )}
                              </div>
                              <div className="min-w-0 flex-1">
                                <div className="flex items-start justify-between gap-2">
                                  <p
                                    className={`truncate text-sm ${scanIssue.readAt ? "font-normal text-default-500" : "font-semibold"}`}
                                  >
                                    {formatScanIssueCode(scanIssue.issueCode)}
                                  </p>
                                  <span className="shrink-0 whitespace-nowrap text-xs text-default-400">
                                    {formatTimeAgo(scanIssue.createdAt)}
                                  </span>
                                </div>
                                <p className="mt-0.5 line-clamp-2 whitespace-normal text-xs text-default-500">
                                  {scanIssue.message}
                                </p>
                                <Chip
                                  size="sm"
                                  variant="flat"
                                  color="warning"
                                  className="mt-2"
                                >
                                  Review scan issue
                                </Chip>
                              </div>
                            </div>
                          </Button>
                        ))}
                      </div>
                    </div>
                  ) : null}

                  {notifications.length > 0 ? (
                    <div>
                      {scanIssues.length > 0 ? (
                        <div className="border-y border-divider bg-default-100/50 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-default-500">
                          Other notifications
                        </div>
                      ) : null}
                      <div className="divide-y divide-divider">
                        {notifications.map((notification) => (
                          <div
                            key={notification.id}
                            className={`cursor-pointer px-4 py-3 transition-colors hover:bg-default-100 ${
                              !notification.readAt ? "bg-primary-50/10" : ""
                            }`}
                            onClick={() =>
                              handleNotificationClick(notification)
                            }
                          >
                            <div className="flex gap-3">
                              <div className="mt-0.5 shrink-0">
                                {getNotificationIcon(
                                  notification.notificationType,
                                )}
                              </div>
                              <div className="min-w-0 flex-1">
                                <div className="flex items-start justify-between gap-2">
                                  <p
                                    className={`text-sm ${
                                      !notification.readAt
                                        ? "font-semibold"
                                        : ""
                                    }`}
                                  >
                                    {notification.title}
                                  </p>
                                  <span className="whitespace-nowrap text-xs text-default-400">
                                    {formatTimeAgo(notification.createdAt)}
                                  </span>
                                </div>
                                <p className="mt-0.5 line-clamp-2 text-xs text-default-500">
                                  {notification.message}
                                </p>

                                {notification.notificationType ===
                                  "ACTION_REQUIRED" &&
                                !notification.resolvedAt ? (
                                  <Chip
                                    size="sm"
                                    variant="flat"
                                    color="secondary"
                                    className="mt-2"
                                  >
                                    Click to resolve
                                  </Chip>
                                ) : null}
                              </div>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  ) : null}
                </div>
              )}
            </ScrollShadow>

            <Divider />
            <div className="px-4 py-2">
              <Button
                size="sm"
                variant="light"
                color="primary"
                className="w-full"
                onPress={handleViewAll}
              >
                View all notifications
              </Button>
            </div>
          </div>
        </PopoverContent>
      </Popover>

      <NotificationDetailModal
        notification={selectedNotification}
        isOpen={isDetailOpen}
        onClose={() => {
          setIsDetailOpen(false);
          setSelectedNotification(null);
        }}
        onResolve={handleResolve}
        onDelete={handleDelete}
        onMarkRead={handleMarkRead}
        onApproveQualityUpgrade={handleApproveQualityUpgrade}
      />
      <ScanIssueDetailModal
        issue={selectedScanIssue}
        isOpen={selectedScanIssue !== null}
        onClose={() => setSelectedScanIssue(null)}
        onChanged={async () => {
          await scanIssuesQuery.refetch();
        }}
      />
    </>
  );
}
