import { describe, expect, it } from "vitest";

import { FINISHED_AT, heardFraction, heardPosition } from "../chapterProgress";

const parts = [{ id: "a", seconds: 100 }, { id: "b", seconds: 100 }, { id: "c", seconds: 100 }];

describe("heardFraction", () => {
  it("measures a chapter inside one long file", () => {
    const place = (offset: number) => ({ fileId: "a", offset });
    expect(heardFraction(parts, place(20), place(60), () => 40)).toBeCloseTo(0.5);
    expect(heardFraction(parts, place(20), place(60), () => 10)).toBe(0);
    expect(heardFraction(parts, place(20), place(60), () => 90)).toBe(1);
  });

  it("follows a chapter across files", () => {
    const positions: Record<string, number> = { a: 100, b: 100, c: 25 };
    expect(heardFraction(parts, { fileId: "a", offset: 50 }, { fileId: "c", offset: 50 }, (id) => positions[id]!)).toBeCloseTo(175 / 200);
  });

  it("doesn't count what was skipped", () => {
    const positions: Record<string, number> = { a: 0, b: 0, c: 90 };
    expect(heardFraction(parts, { fileId: "a", offset: 0 }, { fileId: "b", offset: 50 }, (id) => positions[id]!)).toBe(0);
  });

  it("runs the last chapter to the end of the last file", () => {
    expect(heardFraction(parts, { fileId: "c", offset: 50 }, null, () => 100)).toBeGreaterThanOrEqual(FINISHED_AT);
  });

  it("is zero for a file it doesn't know", () => {
    expect(heardFraction(parts, { fileId: "nope", offset: 0 }, null, () => 100)).toBe(0);
  });
});

describe("heardPosition", () => {
  it("counts a file marked finished by hand as fully heard", () => {
    expect(heardPosition({ currentPosition: 0, isWatched: true }, 100)).toBe(100);
  });
  it("trusts the position of a file past the finished line, which may still have unheard chapters", () => {
    expect(heardPosition({ currentPosition: 92, isWatched: true }, 100)).toBe(92);
  });
  it("is zero with no progress", () => {
    expect(heardPosition(undefined, 100)).toBe(0);
  });
});
