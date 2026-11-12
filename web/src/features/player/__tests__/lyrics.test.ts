import { describe, expect, it } from "vitest";

import { activeLine, lyricsCredit, parseLrc, stanzas } from "../lyrics";

describe("lyrics", () => {
  it("reads stored LRC into sorted timed lines", () => {
    const lines = parseLrc("[00:01.50]One\n[01:02.00]Three\n[00:30.25]Two\nnot a line\n[00:40.00]\n");
    expect(lines).toEqual([
      { start: 1.5, text: "One" },
      { start: 30.25, text: "Two" },
      { start: 40, text: "" },
      { start: 62, text: "Three" },
    ]);
    expect(parseLrc("[00:01.005]x")[0]!.start).toBeCloseTo(1.005);
    expect(parseLrc(null)).toEqual([]);
  });

  it("finds the line being sung", () => {
    const lines = parseLrc("[00:01.00]a\n[00:05.00]b\n[00:09.00]c");
    expect(activeLine(lines, 0)).toBe(-1);
    expect(activeLine(lines, 1)).toBe(0);
    expect(activeLine(lines, 4.99)).toBe(1);
    expect(activeLine(lines, 100)).toBe(2);
    expect(activeLine([], 3)).toBe(-1);
  });

  it("splits plain lyrics into stanzas and credits the source", () => {
    expect(stanzas("a\nb\n\nc\r\n\r\n\r\nd")).toEqual([["a", "b"], ["c"], ["d"]]);
    expect(lyricsCredit("lrclib")).toBe("Lyrics from LRCLIB");
    expect(lyricsCredit("none")).toBeUndefined();
  });
});
