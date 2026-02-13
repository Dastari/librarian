import { createFileRoute } from "@tanstack/react-router";
import { useState, useMemo, useCallback } from "react";
import { Button } from "@heroui/button";
import { Chip } from "@heroui/chip";
import { Checkbox } from "@heroui/checkbox";
import { useDisclosure } from "@heroui/modal";
import { Tabs, Tab } from "@heroui/tabs";
import { addToast } from "@heroui/toast";
import {
  IconTrash,
  IconAlertTriangle,
  IconInfoCircle,
  IconAlertCircle,
  IconBellRinging,
  IconRefresh,
} from "@tabler/icons-react";
import {
  NotificationsDocument,
  NotificationChangedDocument,
  UpdateNotificationDocument,
  DeleteNotificationDocument,
  SortDirection,
  type NotificationsQuery,
  type UpdateNotificationMutation,
  type UpdateNotificationMutationVariables,
  type DeleteNotificationMutation,
  type DeleteNotificationMutationVariables,
} from "../lib/graphql/generated/graphql";
import {
  apolloClient,
  useMutation,
  useQuery,
  useSubscription,
} from "../lib/graphql/client";
import {
  DataTable,
  type DataTableColumn,
  type RowAction,
} from "../components/data-table";
import { NotificationDetailModal } from "../components/NotificationDetailModal";

export const Route = createFileRoute("/notifications")({
  component: NotificationsPage,
});

type NotificationType = "INFO" | "WARNING" | "ERROR" | "ACTION_REQUIRED";
type NotificationCategory =
  | "MATCHING"
  | "PROCESSING"
  | "QUALITY"
  | "STORAGE"
  | "EXTRACTION"
  | "CONFIGURATION";
type NotificationResolution =
  | "ACCEPTED"
  | "REJECTED"
  | "DISMISSED"
  | "AUTO_RESOLVED";

interface NotificationItem {
  id: string;
  title: string;
  message: string;
  notificationType: NotificationType;
  category: NotificationCategory;
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

type NotificationNode = NotificationsQuery["Notifications"]["Edges"][number]["Node"];

function nodeToNotification(node: NotificationNode): NotificationItem {
  let actionData: Record<string, unknown> | null = null;
  if (node.ActionData) {
    try {
      actionData = JSON.parse(node.ActionData) as Record<string, unknown>;
    } catch {
      actionData = null;
    }
  }

  return {
    id: node.Id,
    title: node.Title,
    message: node.Message,
    notificationType: node.NotificationType as NotificationType,
    category: node.Category as NotificationCategory,
    libraryId: node.LibraryId ?? null,
    torrentId: node.TorrentId ?? null,
    mediaFileId: node.MediaFileId ?? null,
    pendingMatchId: node.PendingMatchId ?? null,
    actionType: node.ActionType ?? null,
    actionData,
    readAt: node.ReadAt ?? null,
    resolvedAt: node.ResolvedAt ?? null,
    resolution: (node.Resolution as NotificationResolution) ?? null,
    createdAt: node.CreatedAt,
  };
}

const UNREAD_WHERE = { ReadAt: { IsNull: true } } as const;
const ACTION_REQUIRED_WHERE = {
  NotificationType: { Eq: "ACTION_REQUIRED" },
  ResolvedAt: { IsNull: true },
} as const;
const ORDER_BY_RECENT = [{ CreatedAt: SortDirection.Desc }];
const NOTIFICATIONS_PAGE_SIZE = 50;
const BATCH_PAGE_SIZE = 100;

// Notification type info for display
const NOTIFICATION_TYPE_INFO: Record<
  NotificationType,
  {
    color:
      | "default"
      | "primary"
      | "success"
      | "warning"
      | "danger"
      | "secondary";
    label: string;
  }
> = {
  INFO: { color: "primary", label: "Info" },
  WARNING: { color: "warning", label: "Warning" },
  ERROR: { color: "danger", label: "Error" },
  ACTION_REQUIRED: { color: "secondary", label: "Action Required" },
};

// Category labels
const CATEGORY_LABELS: Record<NotificationCategory, string> = {
  MATCHING: "Matching",
  PROCESSING: "Processing",
  QUALITY: "Quality",
  STORAGE: "Storage",
  EXTRACTION: "Extraction",
  CONFIGURATION: "Configuration",
};

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

function formatTimestamp(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;

  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

type TabKey = "all" | "unread" | "action_required";

function NotificationsPage() {
  const [selectedNotification, setSelectedNotification] =
    useState<NotificationItem | null>(null);
  const [activeTab, setActiveTab] = useState<TabKey>("all");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  const {
    isOpen: isDetailOpen,
    onOpen: onDetailOpen,
    onClose: onDetailClose,
  } = useDisclosure();

  const notificationsFilter = useMemo(() => {
    if (activeTab === "unread") {
      return UNREAD_WHERE;
    }
    if (activeTab === "action_required") {
      return ACTION_REQUIRED_WHERE;
    }
    return undefined;
  }, [activeTab]);

  const notificationsQuery = useQuery(NotificationsDocument, {
    variables: {
      Where: notificationsFilter,
      OrderBy: ORDER_BY_RECENT,
      Page: { Limit: NOTIFICATIONS_PAGE_SIZE, Offset: 0 },
    },
    fetchPolicy: "cache-and-network",
  });

  useSubscription(NotificationChangedDocument, {
    variables: {},
    onData: () => {
      void notificationsQuery.refetch();
    },
  });

  const notifications = useMemo(() => {
    const edges =
      notificationsQuery.data?.Notifications?.Edges ??
      notificationsQuery.previousData?.Notifications?.Edges ??
      [];
    return edges
      .map((edge) => edge?.Node)
      .filter((node): node is NotificationNode => Boolean(node))
      .map((node) => nodeToNotification(node));
  }, [notificationsQuery.data, notificationsQuery.previousData]);
  const totalCount =
    notificationsQuery.data?.Notifications?.PageInfo?.TotalCount ??
    notificationsQuery.previousData?.Notifications?.PageInfo?.TotalCount ??
    0;
  const isLoading = notificationsQuery.loading;

  const [updateNotification] = useMutation<
    UpdateNotificationMutation,
    UpdateNotificationMutationVariables
  >(UpdateNotificationDocument);
  const [deleteNotification] = useMutation<
    DeleteNotificationMutation,
    DeleteNotificationMutationVariables
  >(DeleteNotificationDocument);

  const fetchNotifications = useCallback(() => {
    void notificationsQuery.refetch();
  }, [notificationsQuery]);

  const fetchAllUnreadNotificationIds = useCallback(async (): Promise<string[]> => {
    const ids: string[] = [];
    let offset = 0;

    while (true) {
      const result = await apolloClient.query({
        query: NotificationsDocument,
        variables: {
          Where: UNREAD_WHERE,
          OrderBy: ORDER_BY_RECENT,
          Page: { Limit: BATCH_PAGE_SIZE, Offset: offset },
        },
        fetchPolicy: "network-only",
      });

      const edges = result.data?.Notifications?.Edges ?? [];
      if (edges.length === 0) {
        break;
      }

      ids.push(...edges.map((edge) => edge.Node.Id));

      const hasNextPage = result.data?.Notifications?.PageInfo?.HasNextPage;
      if (!hasNextPage) {
        break;
      }
      offset += BATCH_PAGE_SIZE;
    }

    return ids;
  }, []);

  const handleMarkRead = async (id: string) => {
    try {
      const result = await updateNotification({
        variables: {
          Id: id,
          Input: { ReadAt: new Date().toISOString() },
        },
      });

      if (!result.data?.UpdateNotification.Success) {
        throw new Error(result.data?.UpdateNotification.Error ?? "Mutation failed");
      }

      fetchNotifications();
    } catch (error) {
      addToast({
        title: "Error",
        description: "Failed to mark notification as read",
        color: "danger",
      });
    }
  };

  const handleMarkAllRead = async () => {
    try {
      const unreadIds = await fetchAllUnreadNotificationIds();

      if (unreadIds.length === 0) {
        return;
      }

      const now = new Date().toISOString();
      for (const id of unreadIds) {
        const result = await updateNotification({
          variables: { Id: id, Input: { ReadAt: now } },
        });
        if (!result.data?.UpdateNotification.Success) {
          throw new Error(result.data?.UpdateNotification.Error ?? "Mutation failed");
        }
      }

      fetchNotifications();
      addToast({
        title: "Success",
        description: `Marked ${unreadIds.length} notifications as read`,
        color: "success",
      });
    } catch (error) {
      addToast({
        title: "Error",
        description: "Failed to mark all as read",
        color: "danger",
      });
    }
  };

  const handleResolve = async (
    id: string,
    resolution: NotificationResolution,
  ) => {
    try {
      const now = new Date().toISOString();
      const result = await updateNotification({
        variables: {
          Id: id,
          Input: {
            ResolvedAt: now,
            Resolution: resolution,
            ReadAt: now,
          },
        },
      });

      if (!result.data?.UpdateNotification.Success) {
        throw new Error(result.data?.UpdateNotification.Error ?? "Mutation failed");
      }

      fetchNotifications();

      onDetailClose();

      addToast({
        title: "Notification resolved",
        description: `Action: ${resolution.toLowerCase().replace("_", " ")}`,
        color: "success",
      });
    } catch (error) {
      addToast({
        title: "Error",
        description: "Failed to resolve notification",
        color: "danger",
      });
    }
  };

  const handleDelete = async (id: string) => {
    try {
      const result = await deleteNotification({ variables: { Id: id } });
      if (!result.data?.DeleteNotification.Success) {
        throw new Error(result.data?.DeleteNotification.Error ?? "Mutation failed");
      }
      fetchNotifications();

      addToast({ title: "Notification deleted", color: "success" });
    } catch (error) {
      addToast({
        title: "Error",
        description: "Failed to delete notification",
        color: "danger",
      });
    }
  };

  const handleViewDetails = (notification: NotificationItem) => {
    setSelectedNotification(notification);
    // Mark as read when viewing
    if (!notification.readAt) {
      void handleMarkRead(notification.id);
    }
    onDetailOpen();
  };

  const handleBulkDelete = async () => {
    if (selectedIds.size === 0) return;

    let deletedCount = 0;
    for (const id of selectedIds) {
      try {
        const result = await deleteNotification({ variables: { Id: id } });
        if (result.data?.DeleteNotification.Success) {
          deletedCount++;
        }
      } catch {
        // Continue deleting others
      }
    }

    fetchNotifications();
    setSelectedIds(new Set());

    addToast({
      title: "Deleted",
      description: `Deleted ${deletedCount} notification${deletedCount !== 1 ? 's' : ''}`,
      color: "success",
    });
  };

  const handleSelectAll = () => {
    if (selectedIds.size === notifications.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(notifications.map((n) => n.id)));
    }
  };

  const handleSelectOne = (id: string, checked: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (checked) {
        next.add(id);
      } else {
        next.delete(id);
      }
      return next;
    });
  };

  const columns: DataTableColumn<NotificationItem>[] = [
    {
      key: "select",
      label: (
        <Checkbox
          isSelected={selectedIds.size === notifications.length && notifications.length > 0}
          isIndeterminate={selectedIds.size > 0 && selectedIds.size < notifications.length}
          onValueChange={handleSelectAll}
          aria-label="Select all"
        />
      ) as unknown as string,
      width: 50,
      render: (notification) => (
        <Checkbox
          isSelected={selectedIds.has(notification.id)}
          onValueChange={(checked) => handleSelectOne(notification.id, checked)}
          aria-label={`Select ${notification.title}`}
        />
      ),
    },
    {
      key: "type",
      label: "Type",
      width: 140,
      render: (notification) => (
        <div className="flex items-center gap-2">
          {getNotificationIcon(notification.notificationType)}
          <Chip
            size="sm"
            variant="flat"
            color={NOTIFICATION_TYPE_INFO[notification.notificationType].color}
          >
            {NOTIFICATION_TYPE_INFO[notification.notificationType].label}
          </Chip>
        </div>
      ),
    },
    {
      key: "title",
      label: "Title",
      render: (notification) => (
        <div className="flex flex-col">
          <span
            className={`text-sm ${!notification.readAt ? "font-semibold" : ""}`}
          >
            {notification.title}
          </span>
          <span className="text-xs text-default-500 line-clamp-1">
            {notification.message}
          </span>
        </div>
      ),
    },
    {
      key: "category",
      label: "Category",
      width: 120,
      render: (notification) => (
        <Chip size="sm" variant="flat">
          {CATEGORY_LABELS[notification.category]}
        </Chip>
      ),
    },
    {
      key: "status",
      label: "Status",
      width: 120,
      render: (notification) => {
        if (notification.resolvedAt && notification.resolution) {
          return (
            <Chip
              size="sm"
              variant="flat"
              color={
                notification.resolution === "ACCEPTED"
                  ? "success"
                  : notification.resolution === "REJECTED"
                    ? "danger"
                    : "default"
              }
            >
              {notification.resolution.toLowerCase().replace("_", " ")}
            </Chip>
          );
        }
        if (notification.readAt) {
          return (
            <Chip size="sm" variant="flat" color="default">
              Read
            </Chip>
          );
        }
        return (
          <Chip size="sm" variant="flat" color="primary">
            Unread
          </Chip>
        );
      },
    },
    {
      key: "createdAt",
      label: "Time",
      width: 100,
      render: (notification) => (
        <span className="text-sm text-default-500">
          {formatTimestamp(notification.createdAt)}
        </span>
      ),
    },
  ];

  const rowActions: RowAction<NotificationItem>[] = [
    {
      key: "view",
      label: "View Details",
      onAction: handleViewDetails,
    },
    {
      key: "markRead",
      label: "Mark as Read",
      onAction: (notification: NotificationItem) => handleMarkRead(notification.id),
      isDisabled: (notification: NotificationItem) => !!notification.readAt,
    },
    {
      key: "delete",
      label: "Delete",
      color: "danger",
      onAction: (notification: NotificationItem) => handleDelete(notification.id),
    },
  ];

  const unreadCount = notifications.filter((n) => !n.readAt).length;

  // Header content for the DataTable
  const headerContent = (
    <div className="flex items-center justify-between w-full">
      <div>
        <h1 className="text-2xl font-bold">Notifications</h1>
        <p className="text-default-500">
          {totalCount} total, {unreadCount} unread
        </p>
      </div>
      <div className="flex gap-2">
        {selectedIds.size > 0 && (
          <Button
            color="danger"
            variant="flat"
            startContent={<IconTrash size={16} />}
            onPress={handleBulkDelete}
            aria-label={`Delete ${selectedIds.size} selected notifications`}
          >
            Delete {selectedIds.size} selected
          </Button>
        )}
        <Button
          variant="flat"
          startContent={<IconRefresh size={16} />}
          onPress={fetchNotifications}
          isLoading={isLoading}
          aria-label="Refresh notifications"
        >
          Refresh
        </Button>
        {unreadCount > 0 && (
          <Button color="primary" variant="flat" onPress={handleMarkAllRead}>
            Mark All Read
          </Button>
        )}
      </div>
    </div>
  );

  // Filter row content for tabs
  const filterRowContent = (
    <Tabs
      selectedKey={activeTab}
      onSelectionChange={(key) => setActiveTab(key as TabKey)}
      size="sm"
      variant="underlined"
    >
      <Tab key="all" title="All" />
      <Tab key="unread" title="Unread" />
      <Tab key="action_required" title="Action Required" />
    </Tabs>
  );

  return (
    <div className="container mx-auto p-6 max-w-6xl">
      <DataTable
        stateKey="notifications"
        data={notifications}
        columns={columns}
        rowActions={rowActions}
        onRowClick={handleViewDetails}
        isLoading={isLoading}
        skeletonRowCount={5}
        skeletonDelay={300}
        headerContent={headerContent}
        filterRowContent={filterRowContent}
        hideToolbar
        ariaLabel="Notifications list"
        emptyContent={
          <div className="text-center text-default-500 py-8">
            No notifications
          </div>
        }
        getRowKey={(notification) => notification.id}
      />

      {/* Notification Detail Modal */}
      <NotificationDetailModal
        notification={selectedNotification}
        isOpen={isDetailOpen}
        onClose={onDetailClose}
        onResolve={handleResolve}
        onDelete={handleDelete}
        onMarkRead={handleMarkRead}
      />
    </div>
  );
}
