// @vitest-environment node
import { describe, expect, it } from "vitest";

import {
  episodeRange,
  isScheduleNoise,
  relativeDay,
  releaseBatches,
  scheduledLabel,
  streamingGrid,
  tonightPicks,
  upcomingByShow,
  type GuideEpisode,
  type ScheduleRow,
} from "../guide-model";

const today = new Date(2026, 9, 30);
const percy = { id: "percy", name: "Percy Jackson", network: "Disney+", posterUrl: "p", backdropUrl: "b", autoDownload: true };
const silo = { ...percy, id: "silo", name: "Silo", network: "Apple TV" };

const episode = (id: string, show: typeof percy, season: number, number: number, airDate: string): GuideEpisode => ({

  id,
  showId: show.id,
  season,
  episode: number,
  title: `Episode ${number}`,
  airDate,
  airStamp: `${airDate}T12:00:00+00:00`,
  mediaFileId: null,
  show,
});

const row = (id: number, overrides: Partial<ScheduleRow> = {}): ScheduleRow => ({

  id: `row-${id}`,
  tvmazeEpisodeId: id,
  episodeName: "Episode",
  season: 2,
  episodeNumber: 3,
  episodeType: null,
  airDate: "2026-10-30",
  airTime: null,
  airStamp: null,
  runtime: 50,
  episodeImageUrl: null,
  summary: null,
  tvmazeShowId: id,
  showName: `Show ${id}`,
  showNetwork: "Netflix",
  showPosterUrl: "poster",
  showGenres: ["Drama"],
  countryCode: "US",
  createdAt: "",
  updatedAt: "",
  ...overrides,
});

describe("upcomingByShow", () => {
  it("collapses a show's season into one entry, soonest show first", () => {
    const schedules = upcomingByShow([
      episode("s1", silo, 4, 1, "2027-07-09"),
      episode("p2", percy, 3, 2, "2026-11-20"),
      episode("p1", percy, 3, 1, "2026-11-20"),
      episode("p3", percy, 3, 3, "2026-11-27"),
      episode("old", percy, 2, 8, "2026-10-01"),
    ], today);
    expect(schedules.map((item) => [item.show.name, item.season, item.episodes.map((ep) => ep.episode), item.premiere])).toEqual([
      ["Percy Jackson", 3, [1, 2, 3], true],
      ["Silo", 4, [1], true],
    ]);
  });

  it("keeps only the next episode's season", () => {
    const [next] = upcomingByShow([episode("a", percy, 3, 8, "2026-12-11"), episode("b", percy, 4, 1, "2027-06-01")], today);
    expect(next!.episodes.map((item) => item.id)).toEqual(["a"]);
    expect(next!.premiere).toBe(false);
  });
});

describe("release batches and labels", () => {
  it("groups episodes released the same day", () => {
    const batches = releaseBatches([episode("1", percy, 3, 1, "2026-11-20"), episode("2", percy, 3, 2, "2026-11-20"), episode("3", percy, 3, 3, "2026-11-27")]);
    expect(batches.map((batch) => [batch.date, episodeRange(batch.episodes)])).toEqual([["2026-11-20", "E1–2"], ["2026-11-27", "E3"]]);
  });

  it("describes how far away a day is", () => {
    expect(relativeDay("2026-10-30", today)).toBe("Today");
    expect(relativeDay("2026-10-31", today)).toBe("Tomorrow");
    expect(relativeDay("2026-11-20", today)).toBe("in 3 weeks");
    expect(relativeDay("2027-07-09", today)).toBe("in 8 months");
  });
});

describe("schedule filtering", () => {
  it("treats news, talk, sports and year-numbered daily shows as noise", () => {
    expect(isScheduleNoise(row(1, { season: 2026, episodeNumber: 217, showGenres: [] }))).toBe(true);
    expect(isScheduleNoise(row(2, { showGenres: ["Sports"] }))).toBe(true);
    expect(isScheduleNoise(row(3, { showGenres: [], episodeNumber: 200 }))).toBe(true);
    expect(isScheduleNoise(row(4))).toBe(false);
  });

  it("puts new series and premieres first in tonight's picks and skips library shows", () => {
    const picks = tonightPicks([
      row(1, { showName: "Tulsa King", season: 4, episodeNumber: 3 }),
      row(2, { showName: "Nocturne", season: 1, episodeNumber: 1 }),
      row(3, { showName: "Nocturne", season: 1, episodeNumber: 2, tvmazeShowId: 2 }),
      row(4, { showName: "Dexter", season: 2, episodeNumber: 1 }),
      row(5, { showName: "Silo", season: 4, episodeNumber: 1 }),
      row(6, { showName: "Morning Joe", season: 2026, episodeNumber: 215, showGenres: [] }),
      row(7, { showName: "Tomorrow", airDate: "2026-10-31" }),
    ], "2026-10-30", new Set(["silo"]));
    expect(picks.map((pick) => [pick.name, scheduledLabel(pick)])).toEqual([
      ["Nocturne", "New series"],
      ["Dexter", "S2 premiere"],
      ["Tulsa King", "S4 E3"],
    ]);
  });

  it("builds a services × days grid, busiest service first", () => {
    const days = ["2026-10-30", "2026-10-31"];
    const grid = streamingGrid([
      row(1, { showNetwork: "Apple TV" }),
      row(2, { showNetwork: "Netflix" }),
      row(3, { showNetwork: "Netflix", airDate: "2026-10-31" }),
      row(4, { showNetwork: "Netflix", season: 2026, episodeNumber: 300, showGenres: [] }),
      row(5, { showNetwork: "Netflix", airDate: "2026-11-05" }),
    ], days, { hideNoise: true, genres: [], newOnly: false });
    expect(grid.map((item) => [item.network, item.total, [...item.days.keys()]])).toEqual([
      ["Netflix", 2, days],
      ["Apple TV", 1, ["2026-10-30"]],
    ]);
    expect(streamingGrid([row(1, { showGenres: ["Comedy"] }), row(2)], days, { hideNoise: true, genres: ["Comedy"], newOnly: false })[0]!.total).toBe(1);
  });
});

describe("services", () => {
  it("recognises subscription streamers and limits the grid to chosen services", async () => {
    const { isStreamingService, scheduleNetworks } = await import("../guide-model");
    expect(["Netflix", "Apple TV", "HBO Max", "Paramount+", "NBC", "iQIYI"].filter(isStreamingService)).toEqual(["Netflix", "Apple TV", "HBO Max", "Paramount+"]);
    const rows = [row(1, { showNetwork: "NBC" }), row(2, { showNetwork: "NBC" }), row(3, { showNetwork: "Netflix" })];
    expect(scheduleNetworks(rows)).toEqual([{ network: "NBC", count: 2 }, { network: "Netflix", count: 1 }]);
    expect(streamingGrid(rows, ["2026-10-30"], { hideNoise: true, genres: [], newOnly: false, services: ["Netflix"] }).map((item) => item.network)).toEqual(["Netflix"]);
  });
});
