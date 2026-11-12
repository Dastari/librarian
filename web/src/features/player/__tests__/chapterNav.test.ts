import { describe, expect, it } from "vitest";

import { chapterTarget, currentChapterTitle, seekSpan } from "../chapterNav";
import type { PlayItem } from "../store";

const item = (id: string, chapters: Array<[number, string]>, runningChapter?: string): PlayItem => ({
  mediaFileId: id, title: "Book", entity: { kind: "chapter", id }, runningChapter,
  chapters: chapters.map(([start, title]) => ({ start, title })),
});
// Chapter Two starts in part 1 and runs through part 2. Chapter Three starts part way into part 3.
const parts = [item("a", [[0, "One"], [80, "Two"]]), item("b", [], "Two"), item("c", [[30, "Three"]], "Two")];

describe("chapterTarget", () => {
  it("goes forward across parts, skipping one with no chapter start", () => {
    expect(chapterTarget(parts, 0, 10, 1)).toMatchObject({ index: 0, start: 80, title: "Two" });
    expect(chapterTarget(parts, 0, 90, 1)).toMatchObject({ index: 2, start: 30, title: "Three" });
    expect(chapterTarget(parts, 1, 50, 1)).toMatchObject({ index: 2, start: 30 });
  });
  it("has nowhere to go after the last chapter", () => {
    expect(chapterTarget(parts, 2, 40, 1)).toBeNull();
  });
  it("goes back to this chapter's start, then the one before", () => {
    expect(chapterTarget(parts, 0, 90, -1)).toMatchObject({ index: 0, start: 80 });
    expect(chapterTarget(parts, 0, 81, -1)).toMatchObject({ index: 0, start: 0 });
    expect(chapterTarget(parts, 2, 31, -1)).toMatchObject({ index: 0, start: 80 });
  });
  it("is null for items with no chapters", () => {
    expect(chapterTarget([{ mediaFileId: "x", title: "Song", entity: { kind: "track", id: "x" } }], 0, 5, 1)).toBeNull();
  });
});

describe("currentChapterTitle", () => {
  it("names the chapter playing, including one carried over from the part before", () => {
    expect(currentChapterTitle(parts[0], 90)).toBe("Two");
    expect(currentChapterTitle(parts[1], 10)).toBe("Two");
    expect(currentChapterTitle(parts[2], 10)).toBe("Two");
    expect(currentChapterTitle(parts[2], 40)).toBe("Three");
  });
});

describe("the seek bar's span", () => {
  it("covers the playing chapter of a book, not the whole file", () => {
    const book = item("a", [[0, "One"], [600, "Two"], [1500, "Three"]]);
    expect(seekSpan(book, 700, 2000)).toEqual({ start: 600, end: 1500, chapter: 1, chapters: 3 });
    expect(seekSpan(book, 1800, 2000)).toEqual({ start: 1500, end: 2000, chapter: 2, chapters: 3 });
  });

  it("covers the whole item without chapters to divide it", () => {
    expect(seekSpan(item("b", []), 30, 200)).toEqual({ start: 0, end: 200, chapter: null, chapters: 0 });
    expect(seekSpan(item("c", [[30, "Only"]]), 40, 200)).toEqual({ start: 0, end: 200, chapter: null, chapters: 0 });
  });
});
