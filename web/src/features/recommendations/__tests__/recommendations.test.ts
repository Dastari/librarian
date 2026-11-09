import { describe, expect, it } from "vitest";

import { coverageLabel, lookupInput, recommendationFilter, scopeLabel, targetKey, visibleRows, type Recommendation } from "../recommendations";

const row = (extra: Partial<Recommendation>): Recommendation => ({ id: "r", status: "ready", scope: "season_pack", coversCount: 8, missingCount: 10, episodeId: null, reasons: [], title: "Show.S02.1080p", ...extra }) as Recommendation;

describe("recommendation targets", () => {
  it("filters on the id of what is missing, and on the season for a show", () => {
    expect(recommendationFilter({ movieId: "m" })).toEqual({ movieId: { eq: "m" } });
    expect(recommendationFilter({ albumId: "a" })).toEqual({ albumId: { eq: "a" } });
    expect(recommendationFilter({ audiobookId: "b" })).toEqual({ audiobookId: { eq: "b" } });
    expect(recommendationFilter({ showId: "s", season: 2 })).toEqual({ showId: { eq: "s" }, season: { eq: 2 } });
    expect(recommendationFilter({ showId: "s", series: true })).toEqual({ showId: { eq: "s" }, scope: { eq: "series_pack" } });
    expect(recommendationFilter({ showId: "s" })).toBeNull();
  });

  it("asks for a lookup of exactly the thing opened", () => {
    expect(lookupInput({ showId: "s", season: 2 })).toEqual({ showId: "s", season: 2 });
    expect(lookupInput({ showId: "s", series: true })).toEqual({ showId: "s" });
    expect(lookupInput({ movieId: "m" })).toEqual({ movieId: "m" });
    expect(lookupInput({})).toBeNull();
  });

  it("keys a target so one lookup is asked for once", () => {
    expect(targetKey({ showId: "s", season: 2 })).toBe("s:2");
    expect(targetKey({ showId: "s", series: true })).toBe("s:series");
    expect(targetKey({ movieId: "m" })).toBe("m");
  });
});

describe("what a recommendation says", () => {
  it("says how much of the gap it covers, but not for a single missing item", () => {
    expect(coverageLabel({ coversCount: 8, missingCount: 10 })).toBe("covers 8 of 10 missing");
    expect(coverageLabel({ coversCount: 1, missingCount: 1 })).toBeNull();
    expect(coverageLabel({ coversCount: 0, missingCount: 4 })).toBeNull();
  });

  it("labels each scope", () => {
    expect(scopeLabel("series_pack")).toBe("Complete series");
    expect(scopeLabel("season_pack")).toBe("Season pack");
    expect(scopeLabel("whatever")).toBe("whatever");
  });

  it("shows ready rows, the one covering most first, and only otherwise a placeholder or a none", () => {
    const shown = visibleRows([row({ id: "a", coversCount: 1, scope: "episode" }), row({ id: "b", coversCount: 6 }), row({ id: "c", status: "none" })]);
    expect(shown.ready.map((item) => item.id)).toEqual(["b", "a"]);
    expect(shown.searching).toBe(false);
    expect(shown.none).toBeNull();
    expect(visibleRows([row({ status: "searching" })])).toMatchObject({ ready: [], searching: true });
    expect(visibleRows([row({ status: "none" })]).none?.status).toBe("none");
    expect(visibleRows([])).toEqual({ ready: [], searching: false, none: null });
  });
});
