/**
 * The Wanted page shows one row per title (a show, movie, album or audiobook) with the
 * individual episodes, tracks or chapters folded into it. This module is pure so the grouping
 * can be unit tested.
 */

export type WantedKind = "episode" | "movie" | "track" | "chapter";

export interface WantedItem {
  id: string;
  /** Short label inside the group: "S01E03", "5". */
  code: string;
  title: string;
  season?: number | null;
  wanted: boolean;
  ignored?: boolean | null;
}

export interface WantedGroupMeta {
  key: string;
  kind: WantedKind;
  /** Show, movie, album or audiobook id. */
  parentId: string;
  title: string;
  subtitle: string | null;
  poster: string | null;
  /** Albums use a 1:1 cover, everything else a 2:3 poster. */
  square: boolean;
  libraryId: string;
  imdbId?: string | null;
  year?: number | null;
}

export interface WantedGroup extends WantedGroupMeta {
  items: WantedItem[];
}

export interface WantedEntry {
  group: WantedGroupMeta;
  item: WantedItem;
}

export function groupKey(kind: WantedKind, parentId: string): string {
  return `${kind}:${parentId}`;
}

/** Merges entries into one row per title, keeping item order and sorting rows by title. */
export function buildGroups(entries: WantedEntry[]): WantedGroup[] {
  const groups = new Map<string, WantedGroup>();
  for (const entry of entries) {
    const existing = groups.get(entry.group.key);
    if (existing) existing.items.push(entry.item);
    else groups.set(entry.group.key, { ...entry.group, items: [entry.item] });
  }
  return [...groups.values()].sort((a, b) => a.title.localeCompare(b.title));
}

/** "S01E01, S01E02 +3 more" — the one-line summary under a group title. */
export function summariseItems(items: WantedItem[], max = 4): string {
  const shown = items.slice(0, max).map((item) => item.code).join(", ");
  return items.length > max ? `${shown} +${items.length - max} more` : shown;
}

/** The season shared by every item in the group, or null when they span seasons. */
export function commonSeason(items: WantedItem[]): number | null {
  const seasons = new Set(items.map((item) => item.season).filter((season): season is number => typeof season === "number"));
  return seasons.size === 1 ? [...seasons][0]! : null;
}

export function episodeCode(season: number, episode: number): string {
  return `S${String(season).padStart(2, "0")}E${String(episode).padStart(2, "0")}`;
}
