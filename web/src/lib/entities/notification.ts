import type { BadgeTone, StatusKind } from "glass-ux/badge";

import { entityPath } from "./kinds";
import { libraryRef, linkedItem, mediaFileRef, type ItemLinks, type LinkedRef } from "./linked";

interface Level { label: string; tone: BadgeTone; status: StatusKind; glyphTone?: "info" | "warning" | "danger" }

const LEVEL: Record<string, Level> = {
  INFO: { label: "Info", tone: "info", status: "online", glyphTone: "info" },
  SUCCESS: { label: "Done", tone: "success", status: "online" },
  WARNING: { label: "Warning", tone: "warning", status: "warning", glyphTone: "warning" },
  ERROR: { label: "Error", tone: "danger", status: "error", glyphTone: "danger" },
  ACTION_REQUIRED: { label: "Action needed", tone: "accent", status: "warning", glyphTone: "warning" },
};

/** `Notification.notificationType` as a badge tone, a status dot and a glyph tint. */
export function notificationLevel(type: string | null | undefined): Level {
  return LEVEL[type ?? ""] ?? { label: type || "Info", tone: "neutral", status: "offline" };
}

export const NOTIFICATION_LEVELS = Object.keys(LEVEL);

/** The decision a notification asks for, and how it ended. */
export function notificationDecision(n: { actionType?: string | null; resolvedAt?: string | null; resolution?: string | null }): { label: string; tone: BadgeTone } | null {
  if (!n.actionType) return null;
  if (!n.resolvedAt) return { label: n.actionType === "quality_upgrade" ? "Approve upgrade?" : "Needs a decision", tone: "accent" };
  if (n.resolution === "ACCEPTED" || n.resolution === "approved") return { label: "Approved", tone: "success" };
  return { label: n.resolution === "dismissed" ? "Dismissed" : "Resolved", tone: "neutral" };
}

export interface NotificationLinks {
  library?: { id: string; name: string } | null;
  torrent?: { id: string; name: string } | null;
  mediaFile?: ({ id: string; libraryId?: string | null; path: string } & ItemLinks) | null;
}

/**
 * What a notification is about, most specific first: the item its file belongs to, the
 * file, the download, then the library. The table shows the first; the detail all of them.
 */
export function notificationSubjects(n: NotificationLinks): LinkedRef[] {
  const refs: LinkedRef[] = [];
  if (n.mediaFile) {
    const item = linkedItem(n.mediaFile);
    if (item) refs.push(item);
    refs.push(mediaFileRef(n.mediaFile));
  }
  if (n.torrent) refs.push({ kind: "torrent", id: n.torrent.id, label: n.torrent.name, href: entityPath("torrent", n.torrent.id) });
  if (n.library) refs.push(libraryRef(n.library));
  return refs;
}

/** Quality-upgrade decisions carry the two resolutions in `actionData`. */
export function upgradeDetails(actionData: string | null | undefined): { from?: string; to?: string; sourcePath?: string; targetPath?: string } | null {
  if (!actionData) return null;
  try {
    const data = JSON.parse(actionData) as Record<string, unknown>;
    const text = (value: unknown) => (typeof value === "string" && value ? value : undefined);
    return { from: text(data.existingResolution), to: text(data.newResolution), sourcePath: text(data.sourcePath), targetPath: text(data.targetPath) };
  } catch {
    return null;
  }
}
