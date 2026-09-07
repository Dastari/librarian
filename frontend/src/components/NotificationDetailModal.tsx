import { formatDateTime } from "../lib/format";
import { useNavigate } from "@tanstack/react-router";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { Button } from "@heroui/button";
import { Chip } from "@heroui/chip";
import { Divider } from "@heroui/divider";
import {
  IconCheck,
  IconX,
  IconTrash,
  IconAlertTriangle,
  IconInfoCircle,
  IconAlertCircle,
  IconBellRinging,
  IconSettings,
  IconExternalLink,
  IconFolder,
  IconMovie,
  IconMusic,
  IconLink,
} from "@tabler/icons-react";
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
  notificationType: string;
  category: string;
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

interface NotificationDetailModalProps {
  notification: NotificationItem | null;
  isOpen: boolean;
  onClose: () => void;
  onResolve: (id: string, resolution: NotificationResolution) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
  onMarkRead?: (id: string) => Promise<void>;
  /** Q38: approve a quality-upgrade notification (`approveQualityUpgrade`
   * mutation). Only rendered when `actionType === "quality_upgrade"`. */
  onApproveQualityUpgrade?: (id: string) => Promise<void>;
}

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

const FALLBACK_NOTIFICATION_TYPE_INFO = {
  color: "default" as const,
  label: "Notification",
};

function formatUnknownLabel(value: string | null | undefined): string {
  if (!value) return "Unknown";
  return value
    .toLowerCase()
    .split("_")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function getNotificationTypeInfo(type: string) {
  return (
    NOTIFICATION_TYPE_INFO[type as NotificationType] ?? {
      ...FALLBACK_NOTIFICATION_TYPE_INFO,
      label: formatUnknownLabel(type),
    }
  );
}

// Category labels and settings routes
const CATEGORY_INFO: Record<
  NotificationCategory,
  {
    label: string;
    settingsRoute?: string;
    settingsSection?: string;
    description: string;
  }
> = {
  MATCHING: {
    label: "Matching",
    settingsRoute: "/settings",
    description: "Issues related to matching files to library items",
  },
  PROCESSING: {
    label: "Processing",
    settingsRoute: "/settings",
    description: "Issues during file processing or organization",
  },
  QUALITY: {
    label: "Quality",
    settingsRoute: "/settings",
    description: "Quality verification or upgrade notifications",
  },
  STORAGE: {
    label: "Storage",
    settingsRoute: "/settings",
    settingsSection: "directories",
    description: "Disk space or storage path issues",
  },
  EXTRACTION: {
    label: "Extraction",
    description: "Archive extraction or metadata extraction issues",
  },
  CONFIGURATION: {
    label: "Configuration",
    settingsRoute: "/settings",
    description: "System configuration issues that need attention",
  },
};

function getCategoryInfo(category: string) {
  return (
    CATEGORY_INFO[category as NotificationCategory] ?? {
      label: formatUnknownLabel(category),
      description: "Notification category",
    }
  );
}

interface ResolveAction {
  key: string;
  label: string;
  description: string;
  route?: string;
  section?: string;
  resolution?: NotificationResolution;
  /** Q38: approve a quality-upgrade notification — calls
   * `onApproveQualityUpgrade` (the `approveQualityUpgrade` mutation) instead
   * of the generic `onResolve`/`updateNotification` path, since approving
   * has a real side effect (replacing the file on disk). */
  approveUpgrade?: boolean;
  color?: "primary" | "success" | "warning" | "danger" | "default";
  icon?: React.ReactNode;
}

// Get specific resolve actions based on notification content
function getResolveActions(notification: NotificationItem): ResolveAction[] {
  const actions: ResolveAction[] = [];

  // Check notification title/message for specific actions
  const title = notification.title.toLowerCase();
  const message = notification.message.toLowerCase();

  // Configuration category - check for specific settings
  if (notification.category === "CONFIGURATION") {
    if (
      title.includes("temporary folder") ||
      title.includes("temp folder") ||
      message.includes("temporary folder")
    ) {
      actions.push({
        key: "configure-downloads",
        label: "Configure Download Path",
        description:
          "Go to torrent settings and set a permanent download directory",
        route: "/settings/downloads",
        section: "directories",
        icon: <IconFolder size={16} />,
        color: "primary",
      });
    }

    if (
      title.includes("tmdb") ||
      message.includes("tmdb") ||
      title.includes("movie metadata")
    ) {
      actions.push({
        key: "configure-metadata",
        label: "Configure TMDB API Key",
        description: "Add your TMDB API key in metadata settings",
        route: "/settings/metadata",
        icon: <IconMovie size={16} />,
        color: "primary",
      });
    }

    if (title.includes("port forwarding") || title.includes("upnp")) {
      actions.push({
        key: "configure-ports",
        label: "Configure Port Forwarding",
        description: "View torrent settings for port configuration help",
        route: "/settings/downloads",
        section: "network",
        icon: <IconSettings size={16} />,
        color: "primary",
      });
    }
  }

  // Matching category
  if (notification.category === "MATCHING") {
    if (notification.mediaFileId) {
      actions.push({
        key: "manual-match",
        label: "Manually Match File",
        description: "Open the file matching dialog to link this file",
        route: notification.libraryId
          ? `/libraries/${notification.libraryId}/unmatched`
          : "/downloads",
        icon: <IconLink size={16} />,
        color: "primary",
      });
    }
  }

  // Storage category
  if (notification.category === "STORAGE") {
    actions.push({
      key: "check-storage",
      label: "Check Storage Settings",
      description: "Review library and download path settings",
      route: "/settings/downloads",
      section: "directories",
      icon: <IconFolder size={16} />,
      color: "primary",
    });
  }

  // Quality category
  if (notification.category === "QUALITY") {
    // Q38/Q44: a higher-quality file was found for an already-imported file.
    // Approving replaces the file on disk (never automatic) via the
    // `approveQualityUpgrade` mutation.
    if (notification.actionType === "quality_upgrade") {
      actions.push({
        key: "approve-quality-upgrade",
        label: "Approve Upgrade",
        description: "Replace the existing file with the higher quality version",
        approveUpgrade: true,
        icon: <IconCheck size={16} />,
        color: "success",
      });
    }
    if (notification.libraryId) {
      actions.push({
        key: "view-library",
        label: "View Library",
        description: "Go to the library to review quality settings",
        route: `/libraries/${notification.libraryId}`,
        icon: <IconMusic size={16} />,
        color: "primary",
      });
    }
  }

  // Always add dismiss option
  actions.push({
    key: "dismiss",
    label: "Dismiss",
    description: "Acknowledge this notification without taking action",
    resolution: "DISMISSED",
    color: "default",
    icon: <IconX size={16} />,
  });

  return actions;
}

const getNotificationIcon = (type: string) => {
  switch (type) {
    case "ERROR":
      return <IconAlertCircle size={20} className="text-red-400" />;
    case "WARNING":
      return <IconAlertTriangle size={20} className="text-amber-400" />;
    case "ACTION_REQUIRED":
      return <IconBellRinging size={20} className="text-purple-400" />;
    default:
      return <IconInfoCircle size={20} className="text-blue-400" />;
  }
};

export function NotificationDetailModal({
  notification,
  isOpen,
  onClose,
  onResolve,
  onDelete,
  onMarkRead,
  onApproveQualityUpgrade,
}: NotificationDetailModalProps) {
  const navigate = useNavigate();

  if (!notification) return null;

  const categoryInfo = getCategoryInfo(notification.category);
  const typeInfo = getNotificationTypeInfo(notification.notificationType);
  const resolveActions = notification.resolvedAt
    ? []
    : getResolveActions(notification);

  const handleAction = async (action: ResolveAction) => {
    // Mark as read first if needed
    if (!notification.readAt && onMarkRead) {
      await onMarkRead(notification.id);
    }

    if (action.route) {
      // Navigate to the route
      const url = action.section
        ? `${action.route}?section=${action.section}`
        : action.route;
      onClose();
      navigate({ to: url });
    } else if (action.approveUpgrade) {
      if (onApproveQualityUpgrade) {
        await onApproveQualityUpgrade(notification.id);
      }
      onClose();
    } else if (action.resolution) {
      // Resolve the notification
      await onResolve(notification.id, action.resolution);
      onClose();
    }
  };

  const handleDelete = async () => {
    await onDelete(notification.id);
    onClose();
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="lg">
      <ModalContent>
        <ModalHeader className="flex items-center gap-3">
          {getNotificationIcon(notification.notificationType)}
          <span className="flex-1">{notification.title}</span>
        </ModalHeader>

        <ModalBody className="space-y-4">
          {/* Type and Category chips */}
          <div className="flex gap-2 flex-wrap">
            <Chip size="sm" variant="flat" color={typeInfo.color}>
              {typeInfo.label}
            </Chip>
            <Chip size="sm" variant="flat">
              {categoryInfo.label}
            </Chip>
            {!notification.readAt && (
              <Chip size="sm" variant="flat" color="primary">
                Unread
              </Chip>
            )}
            {notification.resolvedAt && notification.resolution && (
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
            )}
          </div>

          {/* Message */}
          <div className="bg-default-100 rounded-lg p-4">
            <p className="text-default-700 whitespace-pre-wrap">
              {notification.message}
            </p>
          </div>

          {/* Timestamps */}
          <div className="text-sm text-default-500 space-y-1">
            <p>Created: {formatDateTime(notification.createdAt)}</p>
            {notification.readAt && (
              <p>Read: {formatDateTime(notification.readAt)}</p>
            )}
            {notification.resolvedAt && (
              <p>
                Resolved: {formatDateTime(notification.resolvedAt)}
                {notification.resolution &&
                  ` (${notification.resolution.toLowerCase().replace("_", " ")})`}
              </p>
            )}
          </div>

          {/* Action Data (if any) */}
          {notification.actionData &&
            Object.keys(notification.actionData).length > 0 && (
              <div className="bg-default-100 rounded-lg p-4">
                <p className="text-sm font-semibold mb-2">Additional Details</p>
                <pre className="text-xs overflow-auto max-h-32">
                  {JSON.stringify(notification.actionData, null, 2)}
                </pre>
              </div>
            )}

          {/* Resolve Actions */}
          {resolveActions.length > 0 && (
            <>
              <Divider />
              <div className="space-y-3">
                <p className="text-sm font-semibold">Actions</p>
                {resolveActions.map((action) => (
                  <Button
                    key={action.key}
                    variant={action.route ? "flat" : "light"}
                    color={action.color || "default"}
                    size="lg"
                    className="w-full justify-start"
                    startContent={
                      action.icon ||
                      (action.route ? <IconExternalLink size={16} /> : null)
                    }
                    onPress={() => handleAction(action)}
                  >
                    <div className="flex flex-col items-start text-left">
                      <span>{action.label}</span>
                      <span className="text-xs text-default-500 font-normal">
                        {action.description}
                      </span>
                    </div>
                  </Button>
                ))}
              </div>
            </>
          )}

          {/* ACTION_REQUIRED specific buttons (legacy support) */}
          {notification.notificationType === "ACTION_REQUIRED" &&
            !notification.resolvedAt &&
            notification.actionType && (
              <>
                <Divider />
                <div className="flex gap-2">
                  <Button
                    color="success"
                    startContent={<IconCheck size={16} />}
                    onPress={() => onResolve(notification.id, "ACCEPTED")}
                  >
                    Accept
                  </Button>
                  <Button
                    color="danger"
                    variant="flat"
                    startContent={<IconX size={16} />}
                    onPress={() => onResolve(notification.id, "REJECTED")}
                  >
                    Reject
                  </Button>
                </div>
              </>
            )}
        </ModalBody>

        <ModalFooter>
          <Button
            color="danger"
            variant="light"
            startContent={<IconTrash size={16} />}
            onPress={handleDelete}
          >
            Delete
          </Button>
          <Button variant="flat" onPress={onClose}>
            Close
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
