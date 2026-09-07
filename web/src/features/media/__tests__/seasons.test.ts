import { describe, expect, it } from "vitest";

import { groupBySeason, seasonLabel } from "../seasons";

const episode = (season: number, number: number, extra: Partial<{ wanted: boolean; ignored: boolean | null; mediaFileId: string | null }> = {}) => ({
  season,
  episode: number,
  wanted: false,
  ignored: null,
  mediaFileId: null,
  ...extra,
});

describe("season grouping", () => {
  it("groups by season in ascending order and keeps episode order", () => {
    const groups = groupBySeason([episode(2, 1), episode(1, 2), episode(1, 1), episode(0, 1)]);
    expect(groups.map((group) => group.season)).toEqual([0, 1, 2]);
    expect(groups[1]!.episodes.map((item) => item.episode)).toEqual([2, 1]);
    expect(groups[0]!.label).toBe("Specials");
    expect(seasonLabel(3)).toBe("Season 3");
  });

  it("counts have, missing, wanted and ignored", () => {
    const groups = groupBySeason([
      episode(1, 1, { mediaFileId: "file-1" }),
      episode(1, 2, { wanted: true }),
      episode(1, 3),
      episode(1, 4, { ignored: true, wanted: true }),
    ]);
    const [season] = groups;
    expect(season!.total).toBe(4);
    expect(season!.have).toBe(1);
    expect(season!.missing).toBe(2);
    expect(season!.wanted).toBe(1);
    expect(season!.ignored).toBe(1);
    expect(season!.allIgnored).toBe(false);
  });

  it("flags a season that is ignored end to end", () => {
    const groups = groupBySeason([episode(1, 1, { ignored: true }), episode(1, 2, { ignored: true })]);
    expect(groups[0]!.allIgnored).toBe(true);
    expect(groups[0]!.missing).toBe(0);
  });
});
