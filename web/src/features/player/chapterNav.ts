import type { PlayItem } from "./store";

/** A place to skip to: which queue item, and where in it. */
export interface ChapterStop {
  index: number;
  start: number;
  title: string;
}

/** Back goes to the start of this chapter until this far in, and then to the one before. */
const RESTART_WITHIN_SECS = 3;

export function chapterStops(queue: readonly PlayItem[]): ChapterStop[] {
  return queue.flatMap((item, index) => (item.chapters ?? []).map((chapter) => ({ index, start: chapter.start, title: chapter.title })));
}

/** The chapter playing at this position of this item. */
export function currentChapterTitle(item: PlayItem | null | undefined, position: number): string | undefined {
  if (!item?.chapters) return undefined;
  const found = [...item.chapters].reverse().find((chapter) => chapter.start <= position);
  return found?.title ?? item.runningChapter;
}

/** Where a skip lands, or `null` if there's nowhere further. Only for items that have chapters. */
export function chapterTarget(queue: readonly PlayItem[], index: number, position: number, direction: 1 | -1): ChapterStop | null {
  const stops = chapterStops(queue);
  if (!stops.length) return null;
  const isBefore = (stop: ChapterStop, at: number) => stop.index < index || (stop.index === index && stop.start <= at);
  if (direction === 1) return stops.find((stop) => !isBefore(stop, position)) ?? null;
  const past = stops.filter((stop) => isBefore(stop, position));
  const current = past[past.length - 1];
  if (current && current.index === index && position - current.start > RESTART_WITHIN_SECS) return current;
  return past[past.length - 2] ?? current ?? null;
}

/** The stretch of an item the seek bar covers. */
export interface SeekSpan {
  start: number;
  end: number;
  /** Which chapter it is (0-based) and of how many, when the item has chapters. */
  chapter: number | null;
  chapters: number;
}

/**
 * A book's seek bar covers the chapter that's playing, not the whole file: a long file's bar is
 * too coarse to find a place in, and chapters are how people think of a book. Without chapters,
 * the whole item.
 */
export function seekSpan(item: PlayItem | null | undefined, position: number, duration: number): SeekSpan {
  const chapters = [...(item?.chapters ?? [])].sort((a, b) => a.start - b.start);
  if (chapters.length < 2 || !(duration > 0)) return { start: 0, end: duration, chapter: null, chapters: 0 };
  let index = 0;
  for (let at = 0; at < chapters.length; at += 1) if (chapters[at]!.start <= position) index = at;
  const start = Math.max(0, chapters[index]!.start);
  const end = Math.min(duration, chapters[index + 1]?.start ?? duration);
  return end > start ? { start, end, chapter: index, chapters: chapters.length } : { start: 0, end: duration, chapter: null, chapters: 0 };
}
