import { describe, expect, it } from "vitest";

import { buildGroups, commonSeason, episodeCode, groupKey, summariseItems, type WantedEntry } from "../grouping";

const entry = (showId: string, title: string, season: number, number: number): WantedEntry => ({
  group: { key: groupKey("episode", showId), kind: "episode", parentId: showId, title, subtitle: null, poster: null, square: false, libraryId: "lib-1" },
  item: { id: `${showId}-${season}-${number}`, code: episodeCode(season, number), title: `Episode ${number}`, season, wanted: true },
});

describe("wanted grouping", () => {
  it("merges items into one row per title, sorted by title", () => {
    const groups = buildGroups([entry("s2", "Better Call Saul", 1, 1), entry("s1", "Andor", 1, 1), entry("s2", "Better Call Saul", 1, 2)]);
    expect(groups.map((group) => group.title)).toEqual(["Andor", "Better Call Saul"]);
    expect(groups[1]!.items.map((item) => item.code)).toEqual(["S01E01", "S01E02"]);
  });

  it("summarises items and truncates long lists", () => {
    const groups = buildGroups([1, 2, 3, 4, 5, 6].map((number) => entry("s1", "Andor", 1, number)));
    expect(summariseItems(groups[0]!.items)).toBe("S01E01, S01E02, S01E03, S01E04 +2 more");
  });

  it("reports the shared season only when every item agrees", () => {
    const oneSeason = buildGroups([entry("s1", "Andor", 2, 1), entry("s1", "Andor", 2, 2)]);
    expect(commonSeason(oneSeason[0]!.items)).toBe(2);
    const twoSeasons = buildGroups([entry("s1", "Andor", 1, 1), entry("s1", "Andor", 2, 1)]);
    expect(commonSeason(twoSeasons[0]!.items)).toBeNull();
  });
});
