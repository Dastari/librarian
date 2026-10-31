import type { BadgeTone, StatusKind } from "glass-ux/badge";

import { humanize } from "./linked";

interface Tone { label: string; tone: BadgeTone; status: StatusKind; glyphTone?: "info" | "warning" | "danger" }

const SEVERITY: Record<string, Tone> = {
  ERROR: { label: "Error", tone: "danger", status: "error", glyphTone: "danger" },
  WARNING: { label: "Warning", tone: "warning", status: "warning", glyphTone: "warning" },
  INFO: { label: "Info", tone: "info", status: "online", glyphTone: "info" },
};

export function scanSeverity(severity: string | null | undefined): Tone {
  return SEVERITY[severity ?? ""] ?? { label: severity ? humanize(severity) : "Info", tone: "neutral", status: "offline" };
}
export const SCAN_SEVERITIES = Object.keys(SEVERITY);

/** Pipeline stages an issue can come from (library_scan.rs), in pipeline order. */
export const SCAN_STAGES = ["CONFIGURATION", "DISCOVERY", "QUEUE", "ANALYSIS", "ANALYSIS_SETTLE", "PROVIDER_MATCH", "MATCH", "ORGANIZATION", "DUPLICATE_REVIEW", "RECONCILIATION"] as const;
const STAGE_LABEL: Record<string, string> = { ANALYSIS_SETTLE: "Analysis settle", PROVIDER_MATCH: "Provider match", DUPLICATE_REVIEW: "Duplicates" };
export const scanStage = (stage: string) => STAGE_LABEL[stage] ?? humanize(stage);

const ISSUE_LABEL: Record<string, string> = {
  BYTE_IDENTICAL_DUPLICATE: "Duplicate file",
  FFPROBE_UNAVAILABLE: "ffprobe missing",
  FFPROBE_TIMEOUT: "Analysis timed out",
  FFPROBE_MALFORMED_OUTPUT: "Unreadable analysis",
  FFPROBE_OUTPUT_TOO_LARGE: "Analysis too large",
  MEDIA_UNSUPPORTED_OR_CORRUPT: "Unsupported or corrupt",
  MEDIA_PERMISSION_DENIED: "Permission denied",
  TMDB_NOT_CONFIGURED: "TMDB not configured",
  LIBRARY_ROOT_UNAVAILABLE: "Library folder unavailable",
  PROVIDER_REQUEST_FAILED: "Provider request failed",
};
export const issueLabel = (code: string) => ISSUE_LABEL[code] ?? humanize(code);

const RUN: Record<string, Tone> = {
  QUEUED: { label: "Queued", tone: "neutral", status: "offline" },
  RUNNING: { label: "Scanning", tone: "info", status: "installing" },
  COMPLETED: { label: "Completed", tone: "success", status: "online" },
  COMPLETED_WITH_ISSUES: { label: "Found issues", tone: "warning", status: "warning" },
  FAILED: { label: "Failed", tone: "danger", status: "error" },
  CANCELLED: { label: "Cancelled", tone: "neutral", status: "maintenance" },
};
export function scanRunStatus(status: string | null | undefined): Tone {
  return RUN[status ?? ""] ?? { label: status ? humanize(status) : "Unknown", tone: "neutral", status: "offline" };
}
export const SCAN_RUN_STATUSES = Object.keys(RUN);

/** Run duration in words ("4 min", "38 s"); null while it has not started or finished. */
export function scanDuration(run: { startedAt?: string | null; finishedAt?: string | null }): string | null {
  if (!run.startedAt || !run.finishedAt) return null;
  const seconds = Math.max(0, Math.round((Date.parse(run.finishedAt) - Date.parse(run.startedAt)) / 1000));
  if (seconds < 90) return `${seconds} s`;
  const minutes = Math.round(seconds / 60);
  return minutes < 90 ? `${minutes} min` : `${(minutes / 60).toFixed(1)} h`;
}

/** Duplicate and similar issues name two paths in `detailsJson`. */
export function issueDetails(json: string | null | undefined): Record<string, string | number | boolean> | null {
  if (!json) return null;
  try {
    const value = JSON.parse(json) as unknown;
    if (!value || typeof value !== "object" || Array.isArray(value)) return null;
    return Object.fromEntries(Object.entries(value).filter((entry): entry is [string, string | number | boolean] => ["string", "number", "boolean"].includes(typeof entry[1])));
  } catch {
    return null;
  }
}
