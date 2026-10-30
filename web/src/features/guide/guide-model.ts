import type { GuideEpisodesQuery, ScheduleCacheFieldsFragment } from "@/graphql/generated/graphql";

export type GuideEpisode = GuideEpisodesQuery["episodes"]["edges"][number]["node"];
export type GuideShow = NonNullable<GuideEpisode["show"]>;
export type ScheduleRow = ScheduleCacheFieldsFragment;

const DAY = 86_400_000;

/** Local calendar date as `YYYY-MM-DD`; air dates are stored as plain dates. */
export function isoDate(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function startOfToday(now = new Date()): Date {
  const date = new Date(now);
  date.setHours(0, 0, 0, 0);
  return date;
}

export function addDays(date: Date, days: number): Date {
  const next = new Date(date);
  next.setDate(next.getDate() + days);
  return next;
}

/** `2026-11-20` read as a local date (no timezone shift). */
export function parseDay(day: string): Date {
  const [year, month, date] = day.slice(0, 10).split("-").map(Number);
  return new Date(year!, (month ?? 1) - 1, date ?? 1);
}

export function daysUntil(day: string, today: Date): number {
  return Math.round((parseDay(day).getTime() - today.getTime()) / DAY);
}

export const episodeCode = (season: number, episode: number) =>
  episode === 0 ? `S${String(season).padStart(2, "0")} Special` : `S${String(season).padStart(2, "0")}E${String(episode).padStart(2, "0")}`;

/** "E1–2" for a batch released together, "E3" for one. */
export function episodeRange(episodes: Array<{ episode: number }>): string {
  const numbers = episodes.map((item) => item.episode).sort((a, b) => a - b);
  if (numbers.length === 0) return "";
  return numbers.length === 1 || numbers[0] === numbers[numbers.length - 1] ? `E${numbers[0]}` : `E${numbers[0]}–${numbers[numbers.length - 1]}`;
}

/** "Today", "Tomorrow", "in 3 days", "in 3 weeks", "in 8 months". */
export function relativeDay(day: string, today: Date): string {
  const days = daysUntil(day, today);
  if (days <= 0) return "Today";
  if (days === 1) return "Tomorrow";
  if (days < 14) return `in ${days} days`;
  if (days < 60) return `in ${Math.round(days / 7)} weeks`;
  const months = Math.round(days / 30.4);
  return months < 12 ? `in ${months} months` : `in ${Math.round(days / 365)} year${days >= 548 ? "s" : ""}`;
}

export interface ShowSchedule {
  show: GuideShow;
  /** The season of the next episode, and only its upcoming episodes. */
  season: number;
  episodes: GuideEpisode[];
  next: GuideEpisode;
  /** The next episode opens its season. */
  premiere: boolean;
}

/**
 * Upcoming library episodes grouped per show, soonest first. Each show lists the rest of the
 * season its next episode belongs to, so a show is one entry however many episodes it has.
 */
export function upcomingByShow(episodes: GuideEpisode[], today: Date): ShowSchedule[] {
  const from = isoDate(today);
  const byShow = new Map<string, GuideEpisode[]>();
  for (const episode of episodes) {
    if (!episode.show || !episode.airDate || episode.airDate.slice(0, 10) < from) continue;
    const list = byShow.get(episode.showId) ?? [];
    list.push(episode);
    byShow.set(episode.showId, list);
  }
  const schedules: ShowSchedule[] = [];
  for (const list of byShow.values()) {
    const sorted = [...list].sort(compareEpisodes);
    const next = sorted[0]!;
    const season = sorted.filter((item) => item.season === next.season);
    schedules.push({ show: next.show!, season: next.season, episodes: season, next, premiere: next.episode === 1 });
  }
  return schedules.sort((a, b) => compareEpisodes(a.next, b.next) || a.show.name.localeCompare(b.show.name));
}

function compareEpisodes(a: GuideEpisode, b: GuideEpisode): number {
  return (a.airDate ?? "").localeCompare(b.airDate ?? "") || (a.airStamp ?? "").localeCompare(b.airStamp ?? "") || a.season - b.season || a.episode - b.episode;
}

export interface ReleaseBatch<T> {
  date: string;
  episodes: T[];
}

/** Episodes released on the same day, in date order: a weekly double bill is one batch. */
export function releaseBatches<T extends { airDate?: string | null }>(episodes: T[]): Array<ReleaseBatch<T>> {
  const batches: Array<ReleaseBatch<T>> = [];
  for (const episode of episodes) {
    const date = (episode.airDate ?? "").slice(0, 10);
    const last = batches[batches.length - 1];
    if (last && last.date === date) last.episodes.push(episode);
    else batches.push({ date, episodes: [episode] });
  }
  return batches;
}

/** Library episodes keyed by air date, for the calendar. */
export function episodesByDay(episodes: GuideEpisode[]): Map<string, GuideEpisode[]> {
  const days = new Map<string, GuideEpisode[]>();
  for (const episode of [...episodes].sort(compareEpisodes)) {
    const day = (episode.airDate ?? "").slice(0, 10);
    if (!day) continue;
    days.set(day, [...(days.get(day) ?? []), episode]);
  }
  return days;
}

/** One show's episodes on one day, so a calendar cell shows a show once. */
export function groupDayByShow(episodes: GuideEpisode[]): Array<{ show: GuideShow; episodes: GuideEpisode[] }> {
  const groups = new Map<string, { show: GuideShow; episodes: GuideEpisode[] }>();
  for (const episode of episodes) {
    if (!episode.show) continue;
    const group = groups.get(episode.showId) ?? { show: episode.show, episodes: [] };
    group.episodes.push(episode);
    groups.set(episode.showId, group);
  }
  return [...groups.values()];
}

const NOISE_GENRES = new Set(["news", "talk show", "game show", "sports"]);

/**
 * News, talk, daily and sports listings that crowd the full TVmaze schedule. Daily shows
 * number their seasons by year (2026) with hundreds of episodes.
 */
export function isScheduleNoise(row: Pick<ScheduleRow, "season" | "episodeNumber" | "showGenres">): boolean {
  if (row.season >= 1900) return true;
  if (row.showGenres.some((genre) => NOISE_GENRES.has(genre.toLowerCase()))) return true;
  return row.showGenres.length === 0 && row.episodeNumber > 60;
}

export interface ScheduledShow {
  key: string;
  tvmazeShowId: number;
  name: string;
  network: string | null;
  genres: string[];
  posterUrl: string | null;
  season: number;
  episodes: ScheduleRow[];
  /** The first listed episode of a season. */
  premiere: boolean;
  /** Season 1, episode 1. */
  newShow: boolean;
}

/** A day's schedule rows grouped per show (a binge drop is one entry), in listing order. */
export function groupScheduleByShow(rows: ScheduleRow[]): ScheduledShow[] {
  const shows = new Map<number, ScheduledShow>();
  for (const row of rows) {
    const existing = shows.get(row.tvmazeShowId);
    if (existing) {
      if (!existing.episodes.some((item) => item.tvmazeEpisodeId === row.tvmazeEpisodeId)) existing.episodes.push(row);
      existing.premiere ||= row.episodeNumber === 1;
      existing.newShow ||= row.season === 1 && row.episodeNumber === 1;
      continue;
    }
    shows.set(row.tvmazeShowId, {
      key: `${row.airDate}:${row.tvmazeShowId}`,
      tvmazeShowId: row.tvmazeShowId,
      name: row.showName,
      network: row.showNetwork,
      genres: row.showGenres,
      posterUrl: row.showPosterUrl,
      season: row.season,
      episodes: [row],
      premiere: row.episodeNumber === 1,
      newShow: row.season === 1 && row.episodeNumber === 1,
    });
  }
  for (const show of shows.values()) show.episodes.sort((a, b) => a.season - b.season || a.episodeNumber - b.episodeNumber);
  return [...shows.values()];
}

/** "New series", "S2 premiere", "4 episodes", "S4 E3". */
export function scheduledLabel(show: ScheduledShow): string {
  if (show.newShow) return "New series";
  if (show.premiere) return `S${show.season} premiere`;
  if (show.episodes.length > 1) return `${show.episodes.length} episodes`;
  const episode = show.episodes[0]!;
  return `S${episode.season} E${episode.episodeNumber}`;
}

/**
 * Tonight's picks from the full schedule: no noise and nothing already in the library, with
 * new series and season premieres first, then shows with posters.
 */
export function tonightPicks(rows: ScheduleRow[], day: string, libraryShowNames: Set<string>, limit = 12): ScheduledShow[] {
  const today = rows.filter((row) => row.airDate.slice(0, 10) === day && !isScheduleNoise(row) && !libraryShowNames.has(row.showName.toLowerCase()));
  const rank = (show: ScheduledShow) => (show.newShow ? 0 : show.premiere ? 1 : 2) + (show.posterUrl ? 0 : 3);
  return groupScheduleByShow(today)
    .sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name))
    .slice(0, limit);
}

export interface GridFilters {
  hideNoise: boolean;
  genres: string[];
  newOnly: boolean;
  /** Networks to show; empty shows every network. */
  services?: string[];
}

/** Subscription streaming services, used to pick a sensible default service filter. */
const STREAMING_SERVICES = ["netflix", "apple tv", "disney+", "prime video", "amazon", "hbo", "max", "paramount+", "hulu", "peacock", "starz", "amc+", "showtime", "mgm+", "britbox", "acorn tv", "shudder", "crunchyroll"];

export function isStreamingService(network: string): boolean {
  const name = network.toLowerCase();
  return STREAMING_SERVICES.some((service) => name === service || name.startsWith(`${service} `));
}

/** Networks in the schedule with how many shows they list, busiest first (noise excluded). */
export function scheduleNetworks(rows: ScheduleRow[]): Array<{ network: string; count: number }> {
  const shows = new Map<string, Set<number>>();
  for (const row of rows) {
    if (isScheduleNoise(row) || !row.showNetwork) continue;
    shows.set(row.showNetwork, (shows.get(row.showNetwork) ?? new Set()).add(row.tvmazeShowId));
  }
  return [...shows.entries()].map(([network, set]) => ({ network, count: set.size })).sort((a, b) => b.count - a.count || a.network.localeCompare(b.network));
}

export interface GridRow {
  network: string;
  /** Shows per day, keyed by ISO date. */
  days: Map<string, ScheduledShow[]>;
  total: number;
}

export function matchesFilters(row: ScheduleRow, filters: GridFilters): boolean {
  if (filters.services?.length && !filters.services.includes(row.showNetwork ?? "Other")) return false;
  if (filters.hideNoise && isScheduleNoise(row)) return false;
  if (filters.genres.length && !row.showGenres.some((genre) => filters.genres.includes(genre))) return false;
  if (filters.newOnly && row.episodeNumber !== 1) return false;
  return true;
}

/** Services (networks) as rows, days as columns; busiest services first. */
export function streamingGrid(rows: ScheduleRow[], days: string[], filters: GridFilters, exclude: Set<string> = new Set()): GridRow[] {
  const inRange = new Set(days);
  const byNetwork = new Map<string, ScheduleRow[]>();
  for (const row of rows) {
    const day = row.airDate.slice(0, 10);
    if (!inRange.has(day) || !matchesFilters(row, filters) || exclude.has(row.showName.toLowerCase())) continue;
    const network = row.showNetwork ?? "Other";
    byNetwork.set(network, [...(byNetwork.get(network) ?? []), row]);
  }
  const grid: GridRow[] = [];
  for (const [network, list] of byNetwork) {
    const perDay = new Map<string, ScheduledShow[]>();
    let total = 0;
    for (const day of days) {
      const shows = groupScheduleByShow(list.filter((row) => row.airDate.slice(0, 10) === day));
      if (shows.length) perDay.set(day, shows);
      total += shows.length;
    }
    grid.push({ network, days: perDay, total });
  }
  return grid.sort((a, b) => b.total - a.total || a.network.localeCompare(b.network));
}

/** Genres present in the schedule, most common first (for filter chips). */
export function scheduleGenres(rows: ScheduleRow[], limit = 8): string[] {
  const counts = new Map<string, number>();
  for (const row of rows) {
    if (isScheduleNoise(row)) continue;
    for (const genre of row.showGenres) counts.set(genre, (counts.get(genre) ?? 0) + 1);
  }
  return [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).slice(0, limit).map(([genre]) => genre);
}
