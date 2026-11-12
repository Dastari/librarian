import { describe, expect, it } from "vitest";

import { appendPosition, appendPositions, moved, POSITION_SPACING, positionForMove } from "../positions";

const rows = (...positions: number[]) => positions.map((position) => ({ position }));

describe("playlist positions", () => {
  it("appends after the greatest position, whatever the order of the rows", () => {
    expect(appendPosition([])).toBe(0);
    expect(appendPosition(rows(0, 9000, 1024))).toBe(9000 + POSITION_SPACING);
    expect(appendPositions(rows(0, 1024), 3)).toEqual([2048, 3072, 4096]);
  });

  it("places a moved entry halfway between its neighbours when there is room", () => {
    expect(positionForMove(rows(0, 600, 1024), 1)).toBe(512);
    expect(positionForMove(rows(0, 0, 1024), 1)).toBe(512);
  });

  it("goes one step past the last neighbour at the end and one step before the first at the start", () => {
    expect(positionForMove(rows(0, 1024, 0), 2)).toBe(2048);
    expect(positionForMove(rows(0, 4096, 5120), 0)).toBe(3072);
  });

  it("never goes negative at the start, leaving a tie with the first entry for the server to settle", () => {
    expect(positionForMove(rows(0, 0, 1024), 0)).toBe(0);
    expect(positionForMove(rows(500, 0, 1024), 0)).toBe(0);
  });

  it("sends the next neighbour's own position when the gap has closed", () => {
    expect(positionForMove(rows(5, 0, 6), 1)).toBe(6);
    expect(positionForMove(rows(5, 0, 5), 1)).toBe(5);
  });

  it("is zero for a list of one", () => {
    expect(positionForMove(rows(7), 0)).toBe(0);
  });

  it("moves an element within an array without changing the original", () => {
    const original = ["a", "b", "c", "d"];
    expect(moved(original, 3, 0)).toEqual(["d", "a", "b", "c"]);
    expect(moved(original, 0, 2)).toEqual(["b", "c", "a", "d"]);
    expect(moved(original, 9, 0)).toEqual(original);
    expect(original).toEqual(["a", "b", "c", "d"]);
  });
});
