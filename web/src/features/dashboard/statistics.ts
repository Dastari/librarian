import type { PlayStatisticsQuery } from "@/graphql/generated/graphql";

export type PlayStatistics = PlayStatisticsQuery["playStatistics"];
export type TopItem = PlayStatistics["topShows"][number];

export const PERIODS = [
  { value: "7d", label: "7 days", days: 7 },
  { value: "30d", label: "30 days", days: 30 },
  { value: "90d", label: "90 days", days: 90 },
  { value: "1y", label: "Year", days: 365 },
] as const;
export type PeriodKey = (typeof PERIODS)[number]["value"];

export function isPeriod(value: unknown): value is PeriodKey {
  return PERIODS.some((period) => period.value === value);
}

/** The local days of a period ending today: from the start of the first day to now. */
export function periodRange(period: PeriodKey, now = new Date()): { from: string; to: string; utcOffsetMinutes: number } {
  const days = PERIODS.find((entry) => entry.value === period)?.days ?? 30;
  const start = new Date(now);
  start.setHours(0, 0, 0, 0);
  start.setDate(start.getDate() - (days - 1));
  return { from: start.toISOString(), to: now.toISOString(), utcOffsetMinutes: -now.getTimezoneOffset() };
}

/** 9000 → "2h 30m", 2700 → "45m", 30 → "30s", 0 → "0m". */
export function formatPlayed(seconds: number): string {
  const total = Math.max(0, Math.round(seconds));
  if (total > 0 && total < 60) return `${total}s`;
  const minutes = Math.round(total / 60);
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (hours === 0) return `${rest}m`;
  if (hours >= 100) return `${hours.toLocaleString()}h`;
  return rest === 0 ? `${hours}h` : `${hours}h ${rest}m`;
}

/** Hours with one decimal, for chart values. */
export const toHours = (seconds: number) => Math.round((seconds / 3600) * 10) / 10;

/** "Mon 3", "Nov 3": a short label for a day or week bucket (dates are YYYY-MM-DD, local). */
export function bucketLabel(date: string, weekly: boolean, locale?: string): string {
  const [year, month, day] = date.split("-").map(Number);
  const value = new Date(year!, (month ?? 1) - 1, day ?? 1);
  return weekly
    ? value.toLocaleDateString(locale, { month: "short", day: "numeric" })
    : value.toLocaleDateString(locale, { weekday: "short", day: "numeric" });
}

/** Daily columns up to three months; weekly beyond that, so a year is 53 columns, not 365. */
export function timeline(stats: Pick<PlayStatistics, "days" | "weeks">, locale?: string) {
  const weekly = stats.days.length === 0 || stats.days.length > 92;
  const buckets = weekly ? stats.weeks : stats.days;
  return {
    weekly,
    categories: buckets.map((bucket) => bucketLabel(bucket.date, weekly, locale)),
    series: [
      { key: "watched", label: "Watched", data: buckets.map((bucket) => toHours(bucket.secondsWatched)) },
      { key: "listened", label: "Listened", data: buckets.map((bucket) => toHours(bucket.secondsListened)) },
    ],
  };
}

/** A ranked list as chart rows: by time, or by plays when nothing claims time (imported plays). */
export function ranked(items: readonly TopItem[]) {
  const byPlays = items.every((item) => item.seconds === 0);
  return {
    byPlays,
    categories: items.map((item) => item.title),
    series: [{ key: byPlays ? "plays" : "hours", label: byPlays ? "Plays" : "Hours", data: items.map((item) => (byPlays ? item.plays : toHours(item.seconds))) }],
  };
}

/** Whether a period has anything to show. */
export const hasPlays = (stats: Pick<PlayStatistics, "totals">) => stats.totals.plays > 0;
