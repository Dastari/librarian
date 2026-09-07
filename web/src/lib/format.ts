/** Formatting helpers. Every human-readable number in the UI goes through these. */

const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

export function formatBytes(bytes: number | null | undefined, digits = 1): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes === 0) return "0 B";
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), BYTE_UNITS.length - 1);
  const value = bytes / 1024 ** exponent;
  return `${value.toFixed(exponent === 0 ? 0 : digits)} ${BYTE_UNITS[exponent]}`;
}

export function formatSpeed(bytesPerSecond: number | null | undefined): string {
  if (!bytesPerSecond) return "0 B/s";
  return `${formatBytes(bytesPerSecond)}/s`;
}

/** 5400 -> "1:30:00", 125 -> "2:05". */
export function formatClock(totalSeconds: number | null | undefined): string {
  if (totalSeconds === null || totalSeconds === undefined || !Number.isFinite(totalSeconds)) return "0:00";
  const seconds = Math.max(0, Math.floor(totalSeconds));
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  const mm = h > 0 ? String(m).padStart(2, "0") : String(m);
  return h > 0 ? `${h}:${mm}:${String(s).padStart(2, "0")}` : `${mm}:${String(s).padStart(2, "0")}`;
}

/** 5400 -> "1h 30m", 45 -> "45m". Used for runtimes. */
export function formatRuntime(totalSecondsOrMinutes: number | null | undefined, unit: "seconds" | "minutes" = "seconds"): string {
  if (!totalSecondsOrMinutes) return "—";
  const minutes = unit === "seconds" ? Math.round(totalSecondsOrMinutes / 60) : totalSecondsOrMinutes;
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return `${m}m`;
  return m === 0 ? `${h}h` : `${h}h ${m}m`;
}

/** Backend timestamps arrive as ISO strings or Unix seconds encoded as strings. */
export function parseTimestamp(value: string | number | null | undefined): Date | null {
  if (value === null || value === undefined || value === "") return null;
  if (typeof value === "number") return new Date(value < 1e12 ? value * 1000 : value);
  const numeric = Number(value);
  if (Number.isFinite(numeric) && /^\d+$/.test(value.trim())) return new Date(numeric < 1e12 ? numeric * 1000 : numeric);
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

const dateFormatter = new Intl.DateTimeFormat(undefined, { year: "numeric", month: "short", day: "numeric" });
const dateTimeFormatter = new Intl.DateTimeFormat(undefined, {
  year: "numeric",
  month: "short",
  day: "numeric",
  hour: "numeric",
  minute: "2-digit",
});
const relativeFormatter = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

export function formatDate(value: string | number | null | undefined): string {
  const date = parseTimestamp(value);
  return date ? dateFormatter.format(date) : "—";
}

export function formatDateTime(value: string | number | null | undefined): string {
  const date = parseTimestamp(value);
  return date ? dateTimeFormatter.format(date) : "—";
}

export function formatRelative(value: string | number | null | undefined, now = Date.now()): string {
  const date = parseTimestamp(value);
  if (!date) return "—";
  const diff = (date.getTime() - now) / 1000;
  const abs = Math.abs(diff);
  if (abs < 45) return "just now";
  if (abs < 3600) return relativeFormatter.format(Math.round(diff / 60), "minute");
  if (abs < 86400) return relativeFormatter.format(Math.round(diff / 3600), "hour");
  if (abs < 86400 * 30) return relativeFormatter.format(Math.round(diff / 86400), "day");
  if (abs < 86400 * 365) return relativeFormatter.format(Math.round(diff / (86400 * 30)), "month");
  return relativeFormatter.format(Math.round(diff / (86400 * 365)), "year");
}

export function formatPercent(fraction: number | null | undefined, digits = 0): string {
  if (fraction === null || fraction === undefined || !Number.isFinite(fraction)) return "—";
  return `${(fraction * 100).toFixed(digits)}%`;
}

export function formatCount(value: number | null | undefined): string {
  if (value === null || value === undefined) return "—";
  return new Intl.NumberFormat().format(value);
}

export function formatYear(value: string | number | null | undefined): string {
  if (typeof value === "number") return String(value);
  const date = parseTimestamp(value);
  return date ? String(date.getFullYear()) : "";
}

/** Joins the present parts with a middle dot, the meta-line convention used on cards. */
export function joinMeta(...parts: Array<string | number | null | undefined | false>): string {
  return parts.filter((part) => part !== null && part !== undefined && part !== false && part !== "").join(" · ");
}

export function pluralize(count: number, singular: string, plural = `${singular}s`): string {
  return `${formatCount(count)} ${count === 1 ? singular : plural}`;
}

/** Sort-friendly title: drops leading articles so "The Matrix" sorts under M. */
export function sortKey(title: string): string {
  return title.replace(/^(the|a|an)\s+/i, "").toLocaleLowerCase();
}
