import type { AutoDownloadMode } from "@/graphql/generated/graphql";
import type { StatusMeta } from "@/lib/status";

/**
 * How a title acquires new files. Shows, albums and audiobooks use `AutoDownloadMode`;
 * movies use the `monitored` + `wanted` pair. This module is the only place that turns
 * either into words.
 */

export const AUTO_DOWNLOAD_MODE: Record<AutoDownloadMode, StatusMeta> = {
  NONE: { label: "Auto-download off", tone: "default", dot: "bg-muted" },
  WANTED: { label: "Auto-download wanted", tone: "accent", dot: "bg-info" },
  ALL: { label: "Auto-download all", tone: "accent", dot: "bg-info" },
};

export function autoDownloadMeta(mode: AutoDownloadMode | null | undefined): StatusMeta {
  return AUTO_DOWNLOAD_MODE[mode ?? "NONE"];
}

export function movieAcquisitionMeta(movie: { monitored: boolean; wanted: boolean }): StatusMeta {
  if (movie.wanted) return { label: "Wanted", tone: "warning", dot: "bg-warning" };
  if (movie.monitored) return { label: "Monitored", tone: "accent", dot: "bg-info" };
  return { label: "Not monitored", tone: "default", dot: "bg-muted" };
}

export const MODE_SEGMENTS: Array<{ key: AutoDownloadMode; label: string }> = [
  { key: "NONE", label: "Off" },
  { key: "WANTED", label: "Wanted only" },
  { key: "ALL", label: "Everything missing" },
];

export const MODE_HELP: Record<AutoDownloadMode, string> = {
  NONE: "Nothing is downloaded automatically. You can still search for releases by hand.",
  WANTED: "Only items you mark as wanted are searched for and downloaded.",
  ALL: "Every item without a file is searched for and downloaded as it appears.",
};
