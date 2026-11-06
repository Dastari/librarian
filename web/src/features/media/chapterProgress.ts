/** How much of a chapter has been heard, when one file holds many chapters or a chapter spans files. */
export interface SavedPlayback {
  currentPosition: number;
  isWatched: boolean;
}

export interface FileLength {
  id: string;
  seconds: number;
}

export interface ChapterPlace {
  fileId: string;
  offset: number;
}

/** A finished file with no position was marked by hand, so all of it counts as heard. */
export function heardPosition(saved: SavedPlayback | undefined, seconds: number): number {
  if (!saved) return 0;
  return saved.isWatched && saved.currentPosition <= 0 ? seconds : saved.currentPosition;
}

/**
 * The share of the stretch from one place to the next that has been heard, across as many files as
 * it covers. Each file only knows how far it got, so skipping ahead doesn't count what came before.
 */
export function heardFraction(files: readonly FileLength[], from: ChapterPlace, to: ChapterPlace | null, positionOf: (fileId: string) => number): number {
  const first = files.findIndex((file) => file.id === from.fileId);
  if (first < 0) return 0;
  const found = to ? files.findIndex((file) => file.id === to.fileId) : -1;
  const last = found >= 0 ? found : files.length - 1;
  let total = 0;
  let heard = 0;
  for (let index = first; index <= last; index++) {
    const file = files[index]!;
    const start = index === first ? from.offset : 0;
    const end = index === last && to && found >= 0 ? to.offset : file.seconds;
    if (end <= start) continue;
    total += end - start;
    heard += Math.max(0, Math.min(positionOf(file.id), end) - start);
  }
  return total > 0 ? Math.min(1, heard / total) : 0;
}

/** Near enough to the end to count, since playback rarely reports the very last second. */
export const FINISHED_AT = 0.98;
