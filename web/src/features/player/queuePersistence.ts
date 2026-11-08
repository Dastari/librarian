import type { PlayItem } from "./store";

/** A person's queue as saved on this device, so a reload does not lose it. */
export interface SavedQueue {
  items: PlayItem[];
  index: number;
}

const MAX_SAVED = 500;

export const queueKey = (userId: string) => `librarian.player.queue.${userId}`;

const isItem = (value: unknown): value is PlayItem => {
  const item = value as PlayItem | null;
  return Boolean(item && typeof item === "object" && typeof item.mediaFileId === "string" && typeof item.title === "string" && item.entity && typeof item.entity.id === "string" && ["movie", "episode", "track", "chapter"].includes(item.entity.kind));
};

/** The saved queue, or null when there is none, it is unreadable, or storage is refused. */
export function readQueue(userId: string): SavedQueue | null {
  try {
    const stored = window.localStorage.getItem(queueKey(userId));
    if (!stored) return null;
    const parsed = JSON.parse(stored) as { items?: unknown; index?: unknown };
    if (!Array.isArray(parsed.items) || typeof parsed.index !== "number") return null;
    const items = parsed.items.filter(isItem);
    // Entries that no longer validate shift the playing one, so find it again before trusting the index.
    const playing = parsed.items[parsed.index];
    const index = isItem(playing) ? items.indexOf(playing) : Math.min(parsed.index, items.length - 1);
    return items.length && index >= 0 ? { items, index } : null;
  } catch {
    return null;
  }
}

/** Saves the queue, or forgets it when it is empty. A browser refusing storage only costs the reload. */
export function writeQueue(userId: string, items: readonly PlayItem[], index: number): void {
  try {
    if (!items.length || index < 0) {
      window.localStorage.removeItem(queueKey(userId));
      return;
    }
    // A very long queue keeps the playing entry and what follows it.
    const from = items.length > MAX_SAVED ? Math.max(0, Math.min(index, items.length - MAX_SAVED)) : 0;
    window.localStorage.setItem(queueKey(userId), JSON.stringify({ items: items.slice(from, from + MAX_SAVED), index: index - from }));
  } catch {
    // Storage is full or refused.
  }
}
