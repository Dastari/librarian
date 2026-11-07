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
