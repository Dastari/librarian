import { useEffect, useState, type ReactNode } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Popover,
  PopoverTrigger,
  PopoverContent,
} from "@heroui/popover";
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
import { graphqlClient } from "../lib/graphql";
import {
  NotificationsDocument,
  UpdateNotificationDocument,
  DeleteNotificationDocument,
  SortDirection,
} from "../lib/graphql/generated/graphql";
import type { Notification } from "../lib/graphql";
import type { NotificationType, NotificationResolution } from "../lib/graphql";
import { NotificationDetailModal } from "./NotificationDetailModal";
import type { NotificationsQuery } from "../lib/graphql/generated/graphql";

interface NotificationPopoverProps {
  trigger: ReactNode;
}

type NotificationNode = NotificationsQuery["Notifications"]["Edges"][0]["Node"];

function nodeToNotification(node: NotificationNode): Notification {
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
    category: node.Category as Notification["category"],
    libraryId: node.LibraryId ?? null,
    torrentId: node.TorrentId ?? null,
    mediaFileId: node.MediaFileId ?? null,
    pendingMatchId: node.PendingMatchId ?? null,
    actionType: (node.ActionType as Notification["actionType"]) ?? null,
    actionData,
    readAt: node.ReadAt ?? null,
    resolvedAt: node.ResolvedAt ?? null,
    resolution: (node.Resolution as NotificationResolution) ?? null,
    createdAt: node.CreatedAt,
  };
}

const UNREAD_WHERE = { ReadAt: { IsNull: true } } as const;
const RECENT_ORDER: Array<{ CreatedAt: "Asc" | "Desc" }> = [
  { CreatedAt: SortDirection.Desc },
];
const RECENT_PAGE = { Limit: 10, Offset: 0 } as const;

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
  const date = new Date(dateString);
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

export function NotificationPopover({ trigger }: NotificationPopoverProps) {
  const navigate = useNavigate();
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isOpen, setIsOpen] = useState(false);
  const [selectedNotification, setSelectedNotification] =
    useState<Notification | null>(null);
  const [isDetailOpen, setIsDetailOpen] = useState(false);

  useEffect(() => {
    if (!isOpen) return;

    setIsLoading(true);
    graphqlClient
      .query(NotificationsDocument, {
        Where: UNREAD_WHERE,
        OrderBy: RECENT_ORDER,
        Page: RECENT_PAGE,
      })
      .toPromise()
      .then((result) => {
        if (result.data?.Notifications?.Edges) {
          const list = result.data.Notifications.Edges.map((e) =>
            nodeToNotification(e.Node),
          );
          setNotifications(list);
        }
      })
      .finally(() => setIsLoading(false));
  }, [isOpen]);

  const handleMarkRead = async (id: string) => {
    const now = new Date().toISOString();
    await graphqlClient
      .mutation(UpdateNotificationDocument, {
        Id: id,
        Input: { ReadAt: now },
      })
      .toPromise();

    setNotifications((prev) =>
      prev.map((n) =>
        n.id === id ? { ...n, readAt: now } : n,
      ),
    );
  };

  const handleMarkAllRead = async () => {
    const now = new Date().toISOString();
    for (const n of notifications) {
      await graphqlClient
        .mutation(UpdateNotificationDocument, {
          Id: n.id,
          Input: { ReadAt: now },
        })
        .toPromise();
    }
    setNotifications((prev) =>
      prev.map((n) => ({ ...n, readAt: now })),
    );
  };

  const handleResolve = async (
    id: string,
    resolution: NotificationResolution,
  ) => {
    const now = new Date().toISOString();
    await graphqlClient
      .mutation(UpdateNotificationDocument, {
        Id: id,
        Input: {
          ResolvedAt: now,
          Resolution: resolution,
          ReadAt: now,
        },
      })
      .toPromise();

    setNotifications((prev) =>
      prev.map((n) =>
        n.id === id
          ? {
              ...n,
              resolvedAt: now,
              resolution,
              readAt: n.readAt ?? now,
            }
          : n,
      ),
    );
  };

  const handleDelete = async (id: string) => {
    await graphqlClient
      .mutation(DeleteNotificationDocument, { Id: id })
      .toPromise();

    setNotifications((prev) => prev.filter((n) => n.id !== id));
  };

  const handleNotificationClick = (notification: Notification) => {
    if (!notification.readAt) {
      handleMarkRead(notification.id);
    }
    setIsOpen(false);
    setSelectedNotification(notification);
    setIsDetailOpen(true);
  };

  const handleViewAll = () => {
    setIsOpen(false);
    navigate({ to: "/notifications" });
  };

  const unreadCount = notifications.filter((n) => !n.readAt).length;

  return (
    <>
      <Popover
        isOpen={isOpen}
        onOpenChange={setIsOpen}
        placement="bottom-end"
        offset={10}
      >
        <PopoverTrigger>
          <span className="inline-flex">{trigger}</span>
        </PopoverTrigger>
        <PopoverContent className="w-80 p-0">
          <div className="flex flex-col">
            <div className="flex items-center justify-between px-4 py-3 border-b border-divider">
              <h3 className="text-sm font-semibold">Notifications</h3>
              {unreadCount > 0 && (
                <Button
                  size="sm"
                  variant="light"
                  color="primary"
                  onPress={handleMarkAllRead}
                >
                  Mark all read
                </Button>
              )}
            </div>

            <ScrollShadow className="max-h-80">
              {isLoading ? (
                <div className="flex items-center justify-center py-8 text-default-400">
                  Loading...
                </div>
              ) : notifications.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-8 text-default-400">
                  <IconCheck size={32} className="mb-2" />
                  <span className="text-sm">No notifications</span>
                </div>
              ) : (
                <div className="divide-y divide-divider">
                  {notifications.map((notification) => (
                    <div
                      key={notification.id}
                      className={`px-4 py-3 hover:bg-default-100 cursor-pointer transition-colors ${
                        !notification.readAt ? "bg-primary-50/10" : ""
                      }`}
                      onClick={() => handleNotificationClick(notification)}
                    >
                      <div className="flex gap-3">
                        <div className="shrink-0 mt-0.5">
                          {getNotificationIcon(
                            notification.notificationType,
                          )}
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-start justify-between gap-2">
                            <p
                              className={`text-sm ${!notification.readAt ? "font-semibold" : ""}`}
                            >
                              {notification.title}
                            </p>
                            <span className="text-xs text-default-400 whitespace-nowrap">
                              {formatTimeAgo(notification.createdAt)}
                            </span>
                          </div>
                          <p className="text-xs text-default-500 mt-0.5 line-clamp-2">
                            {notification.message}
                          </p>

                          {notification.notificationType === "ACTION_REQUIRED" &&
                            !notification.resolvedAt && (
                              <Chip
                                size="sm"
                                variant="flat"
                                color="secondary"
                                className="mt-2"
                              >
                                Click to resolve
                              </Chip>
                            )}

                          {notification.resolvedAt &&
                            notification.resolution && (
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
                                className="mt-2"
                              >
                                {notification.resolution
                                  .toLowerCase()
                                  .replace("_", " ")}
                              </Chip>
                            )}
                        </div>
                      </div>
                    </div>
                  ))}
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
      />
    </>
  );
}
