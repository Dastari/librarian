import type { ContentStatus } from "@/graphql/generated/graphql";

export type StatusTone = "success" | "warning" | "accent" | "danger" | "default";

export interface StatusMeta {
  label: string;
  tone: StatusTone;
  /** Tailwind class for dots and small indicators. */
  dot: string;
}

/**
 * The backend reduces every entity to one `ContentStatus`. This is the single place that
 * turns it into words and colour; components never branch on the raw enum.
 */
export const CONTENT_STATUS: Record<ContentStatus, StatusMeta> = {
  PLAYING: { label: "Playing", tone: "success", dot: "bg-success" },
  PAUSED: { label: "Paused", tone: "accent", dot: "bg-info" },
  DOWNLOADING: { label: "Downloading", tone: "accent", dot: "bg-info" },
  FAILED: { label: "Failed", tone: "danger", dot: "bg-danger" },
  PROCESSING: { label: "Processing", tone: "accent", dot: "bg-info" },
  UPGRADABLE: { label: "Upgrade available", tone: "warning", dot: "bg-warning" },
  AVAILABLE: { label: "Available", tone: "success", dot: "bg-success" },
  IGNORED: { label: "Ignored", tone: "default", dot: "bg-muted" },
  UPCOMING: { label: "Upcoming", tone: "default", dot: "bg-muted" },
  WANTED: { label: "Wanted", tone: "warning", dot: "bg-warning" },
  MISSING: { label: "Missing", tone: "default", dot: "bg-muted" },
};

export function statusMeta(status: ContentStatus | null | undefined): StatusMeta {
  return status ? CONTENT_STATUS[status] : CONTENT_STATUS.MISSING;
}

/** Torrent client states as reported by librqbit through the backend. */
export const TORRENT_STATE: Record<string, StatusMeta> = {
  downloading: { label: "Downloading", tone: "accent", dot: "bg-info" },
  seeding: { label: "Seeding", tone: "success", dot: "bg-success" },
  completed: { label: "Completed", tone: "success", dot: "bg-success" },
  paused: { label: "Paused", tone: "warning", dot: "bg-warning" },
  queued: { label: "Queued", tone: "default", dot: "bg-muted" },
  checking: { label: "Checking", tone: "accent", dot: "bg-info" },
  initializing: { label: "Starting", tone: "accent", dot: "bg-info" },
  error: { label: "Error", tone: "danger", dot: "bg-danger" },
};

const UNKNOWN_STATUS: StatusMeta = { label: "Unknown", tone: "default", dot: "bg-muted" };

export function torrentState(state: string | null | undefined): StatusMeta {
  return TORRENT_STATE[(state ?? "").toLowerCase()] ?? { ...UNKNOWN_STATUS, label: state ?? "Unknown" };
}

/** Scan run states from `LibraryScanRun.status`. */
export const SCAN_STATUS: Record<string, StatusMeta> = {
  QUEUED: { label: "Queued", tone: "default", dot: "bg-muted" },
  RUNNING: { label: "Scanning", tone: "accent", dot: "bg-info" },
  COMPLETED: { label: "Completed", tone: "success", dot: "bg-success" },
  COMPLETED_WITH_ISSUES: { label: "Needs attention", tone: "warning", dot: "bg-warning" },
  FAILED: { label: "Failed", tone: "danger", dot: "bg-danger" },
  CANCELLED: { label: "Cancelled", tone: "default", dot: "bg-muted" },
};

export function scanStatus(status: string | null | undefined): StatusMeta {
  return SCAN_STATUS[status ?? ""] ?? { ...UNKNOWN_STATUS, label: status ?? "Unknown" };
}

export const NOTIFICATION_TYPE: Record<string, StatusMeta> = {
  INFO: { label: "Info", tone: "accent", dot: "bg-info" },
  WARNING: { label: "Warning", tone: "warning", dot: "bg-warning" },
  ERROR: { label: "Error", tone: "danger", dot: "bg-danger" },
  ACTION_REQUIRED: { label: "Action needed", tone: "warning", dot: "bg-warning" },
  SUCCESS: { label: "Done", tone: "success", dot: "bg-success" },
};

export function notificationType(type: string | null | undefined): StatusMeta {
  return NOTIFICATION_TYPE[type ?? ""] ?? { label: "Info", tone: "accent", dot: "bg-info" };
}

export const QUALITY_STATUS: Record<string, StatusMeta> = {
  optimal: { label: "Optimal", tone: "success", dot: "bg-success" },
  suboptimal: { label: "Below target", tone: "warning", dot: "bg-warning" },
  unknown: { label: "Not evaluated", tone: "default", dot: "bg-muted" },
};

export function qualityStatus(status: string | null | undefined): StatusMeta {
  return QUALITY_STATUS[(status ?? "unknown").toLowerCase()] ?? { label: "Not evaluated", tone: "default", dot: "bg-muted" };
}
