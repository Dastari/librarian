import { describe, expect, it } from "vitest";

import { addsLabel, coverageLabel, coversSeasons, lookupInput, planSummary, planView, recommendationFilter, scopeLabel, targetKey, visibleRows, type Recommendation } from "../recommendations";

const row = (extra: Partial<Recommendation>): Recommendation => ({ id: "r", status: "ready", scope: "season_pack", coversCount: 8, missingCount: 10, episodeId: null, reasons: [], title: "Show.S02.1080p", ...extra }) as Recommendation;

describe("recommendation targets", () => {
  it("filters on the id of what is missing, and on the season for a show", () => {
    expect(recommendationFilter({ movieId: "m" })).toEqual({ movieId: { eq: "m" } });
    expect(recommendationFilter({ albumId: "a" })).toEqual({ albumId: { eq: "a" } });
    expect(recommendationFilter({ audiobookId: "b" })).toEqual({ audiobookId: { eq: "b" } });
    // A show's rows are its plan and the plan's releases, whichever season is open.
    expect(recommendationFilter({ showId: "s", season: 2 })).toEqual({ showId: { eq: "s" } });
    expect(recommendationFilter({ showId: "s", series: true })).toEqual({ showId: { eq: "s" } });
    expect(recommendationFilter({ showId: "s" })).toBeNull();
  });

  it("asks for a lookup of exactly the thing opened", () => {
    expect(lookupInput({ showId: "s", season: 2 })).toEqual({ showId: "s" });
    expect(lookupInput({ showId: "s", series: true })).toEqual({ showId: "s" });
    expect(lookupInput({ movieId: "m" })).toEqual({ movieId: "m" });
    expect(lookupInput({})).toBeNull();
  });

  it("keys a target so one lookup is asked for once", () => {
    expect(targetKey({ showId: "s", season: 2 })).toBe("s:plan");
    expect(targetKey({ showId: "s", series: true })).toBe("s:plan");
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

describe("a show's plan", () => {
  const header = row({ id: "p", scope: "plan", coversCount: 24, missingCount: 44, planId: null });
  const members = [
    row({ id: "b", planId: "p", planPosition: 1, scope: "episode", season: 2, coversSeasons: "[2]", coversCount: 1 }),
    row({ id: "a", planId: "p", planPosition: 0, scope: "series_pack", season: null, coversSeasons: "[1,3]", coversCount: 23 }),
    row({ id: "old", planId: null, scope: "season_pack", season: 2 }),
  ];

  it("orders its releases, narrows them to a season, and leaves out rows that are not in it", () => {
    expect(planView([header, ...members]).releases.map((item) => item.id)).toEqual(["a", "b"]);
    expect(planView([header, ...members], 3).releases.map((item) => item.id)).toEqual(["a"]);
    expect(planView([header, ...members], 2).releases.map((item) => item.id)).toEqual(["b"]);
    expect(planView([header, ...members], 9).plan).toBeNull();
    expect(planView([row({ id: "s", scope: "plan", status: "searching" })])).toMatchObject({ plan: null, searching: true });
    expect(planView([row({ id: "n", scope: "plan", status: "none" })]).none?.id).toBe("n");
    // Rows from before plans existed are not a plan.
    expect(planView([members[2]!])).toEqual({ plan: null, releases: [], searching: false, none: null });
  });

  it("is summed up the way a person says it", () => {
    expect(planSummary(header, 3)).toBe("3 releases cover 24 of 44 missing");
    expect(planSummary({ coversCount: 22, missingCount: 22 }, 1)).toBe("1 release covers 22 of 22 missing");
    expect(addsLabel({ coversCount: 22 })).toBe("adds 22");
    expect(coversSeasons({ coversSeasons: "[1,3]", season: null })).toEqual([1, 3]);
    expect(coversSeasons({ coversSeasons: null, season: 2 })).toEqual([2]);
    expect(coversSeasons({ coversSeasons: "not json", season: null })).toEqual([]);
  });
});
