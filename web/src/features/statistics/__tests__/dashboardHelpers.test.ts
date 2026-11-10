import { describe, expect, it } from "vitest";

import { playbackDetails } from "@/features/player/useProgressSync";

import { historyLine, historyVariables } from "../HistoryPage";
import { describeSession, isLive, methodLabel, playState, qualityLabel, sessionProgress, type NowPlayingSession } from "../nowPlaying";
import { audienceOf, shareChanges } from "../ShareStatisticsDialog";
import { formatPlayed, periodRange, ranked, timeline } from "../statistics";

const session = (extra: Partial<NowPlayingSession> = {}) => ({
  id: "s1", userId: "u1", contentType: "episode", mediaFileId: "f1", episodeId: "e1", movieId: null, trackId: null, audiobookId: null, tvShowId: null, albumId: null, chapterId: null,
  currentPosition: 600, duration: 2400, volume: 1, isMuted: false, isPlaying: true, startedAt: "", lastUpdatedAt: "2026-11-09T12:00:00.000Z", completedAt: null, createdAt: "", updatedAt: "",
  deviceId: "d1", state: "playing", playMethod: "direct", quality: "1080p", bitrate: 8_200_000, endedAt: null, stopRequestedAt: null,
  device: { id: "d1", name: "Living room", formFactor: "tv", platform: "webOS", product: null },
  movie: null, track: null, audiobook: null,
  episode: { id: "e1", showId: "sh1", season: 1, episode: 3, title: "In Perpetuity", show: { id: "sh1", name: "Severance" } },
  ...extra,
}) as unknown as NowPlayingSession;

describe("now playing", () => {
  const now = Date.parse("2026-11-09T12:01:00Z");
  it("shows players that reported in the last few minutes and have not closed", () => {
    expect(isLive(session(), now)).toBe(true);
    expect(isLive(session({ lastUpdatedAt: "2026-11-09T11:50:00.000Z" }), now)).toBe(false);
    expect(isLive(session({ endedAt: "2026-11-09T12:00:30.000Z" }), now)).toBe(false);
  });

  it("names what is playing, how, and how far in", () => {
    expect(describeSession(session())).toMatchObject({ title: "In Perpetuity", subtitle: "Severance · S1 · E3", href: "/shows/sh1" });
    expect(describeSession(session({ episode: null, movie: { id: "m1", title: "Arrival", year: 2016 } } as never))).toMatchObject({ title: "Arrival", subtitle: "2016", href: "/movies/m1" });
    expect(describeSession(session({ episode: null, track: { id: "t1", albumId: "a1", title: "Airbag", artistName: "Radiohead", album: { id: "a1", name: "OK Computer" } } } as never))).toMatchObject({ subtitle: "Radiohead · OK Computer", square: true });
    expect(methodLabel("hls")).toBe("Direct stream");
    expect(methodLabel("transcode")).toBe("Transcoding");
    expect(qualityLabel(session())).toBe("1080p · 8.2 Mbps");
    expect(qualityLabel(session({ quality: "FLAC", bitrate: 900_000 }))).toBe("FLAC · 900 kbps");
    expect(sessionProgress(session())).toBe(0.25);
    expect(sessionProgress(session({ duration: null }))).toBeNull();
  });

  it("says whether it is playing, paused or being stopped", () => {
    expect(playState(session()).label).toBe("Playing");
    expect(playState(session({ state: "paused", isPlaying: false })).label).toBe("Paused");
    expect(playState(session({ state: null, isPlaying: false })).label).toBe("Paused");
    expect(playState(session({ stopRequestedAt: "2026-11-09T12:00:10.000Z" })).label).toBe("Stopping");
  });

  it("reports the play method from the media source", () => {
    expect(playbackDetails({ isHls: false, info: { transcode_decision: "none", resolution: "1080p", size_bytes: 1_000_000_000, duration: 1000 } })).toEqual({ method: "direct", quality: "1080p", bitrate: 8_000_000 });
    expect(playbackDetails({ isHls: true, info: { transcode_decision: "remux", resolution: "4K" } })?.method).toBe("hls");
    expect(playbackDetails({ isHls: true, info: { transcode_decision: "transcode", resolution: null, audio_codec: "flac" } })).toMatchObject({ method: "transcode", quality: "FLAC", bitrate: null });
    expect(playbackDetails(null)).toBeNull();
  });
});

describe("statistics", () => {
  it("turns seconds into hours and minutes", () => {
    expect(formatPlayed(0)).toBe("0m");
    expect(formatPlayed(30)).toBe("30s");
    expect(formatPlayed(2700)).toBe("45m");
    expect(formatPlayed(9000)).toBe("2h 30m");
    expect(formatPlayed(7200)).toBe("2h");
  });

  it("asks for whole local days ending today", () => {
    const range = periodRange("7d", new Date(2026, 10, 9, 15, 30));
    const from = new Date(range.from);
    expect(from.getDate()).toBe(3);
    expect(from.getHours()).toBe(0);
    expect(range.utcOffsetMinutes).toBe(-new Date(2026, 10, 9).getTimezoneOffset());
  });

  it("charts days up to three months and weeks after", () => {
    const day = (date: string, watched: number) => ({ date, secondsWatched: watched, secondsListened: 1800, plays: 1 });
    const short = timeline({ days: [day("2026-11-02", 3600), day("2026-11-03", 5400)], weeks: [day("2026-11-02", 9000)] });
    expect(short.weekly).toBe(false);
    expect(short.series[0]!.data).toEqual([1, 1.5]);
    expect(short.series[1]!.data).toEqual([0.5, 0.5]);
    const long = timeline({ days: Array.from({ length: 120 }, (_, i) => day(`2026-01-${String((i % 28) + 1).padStart(2, "0")}`, 0)), weeks: [day("2026-01-05", 3600)] });
    expect(long.weekly).toBe(true);
    expect(long.categories).toHaveLength(1);
  });

  it("ranks by hours, or by plays when only imported plays are known", () => {
    const item = (title: string, seconds: number, plays: number) => ({ key: title, kind: "show", entityId: null, title, subtitle: null, seconds, plays });
    expect(ranked([item("A", 7200, 2), item("B", 3600, 5)]).series[0]).toMatchObject({ key: "hours", data: [2, 1] });
    expect(ranked([item("A", 0, 2), item("B", 0, 5)]).series[0]).toMatchObject({ key: "plays", data: [2, 5] });
  });
});

describe("history", () => {
  it("describes a play in one line", () => {
    expect(historyLine({ contentType: "episode", parentTitle: "Severance", subtitle: "S1 · E3" })).toBe("Severance · S1 · E3");
    expect(historyLine({ contentType: "movie", parentTitle: null, subtitle: "2016" })).toBe("2016");
    expect(historyLine({ contentType: "track", parentTitle: null, subtitle: null })).toBe("Track");
  });

  it("sends the table's filters, search and order to the server", () => {
    const variables = historyVariables({ pagination: { pageIndex: 0, pageSize: 50 }, selection: {}, sorting: [{ id: "played", desc: true }], filters: [{ id: "kind", value: { values: ["movie"] } }], search: "dune" } as never);
    expect(variables.orderBy).toEqual([{ secondsPlayed: "DESC" }]);
    expect(variables.where).toEqual({ and: [{ contentType: { inList: ["movie"] } }, { title: { contains: "dune" } }] });
    expect(historyVariables({ sorting: [], filters: [] } as never)).toEqual({ where: {}, orderBy: [{ startedAt: "DESC" }] });
  });
});

describe("sharing statistics", () => {
  const share = (id: string, audience: string, viewerUserId: string | null = null) => ({ id, audience, viewerUserId });

  it("reads the saved shares as private, everyone or some people", () => {
    expect(audienceOf([])).toEqual({ audience: "private", people: [] });
    expect(audienceOf([share("1", "user", "bob"), share("2", "everyone")]).audience).toBe("everyone");
    expect(audienceOf([share("1", "user", "bob"), share("2", "user", "amy")])).toEqual({ audience: "people", people: ["bob", "amy"] });
  });

  it("changes only what differs", () => {
    const saved = [share("1", "user", "bob"), share("2", "user", "amy")];
    expect(shareChanges(saved, "people", ["amy", "cat"])).toEqual({ remove: ["1"], add: [{ audience: "user", viewerUserId: "cat" }] });
    expect(shareChanges(saved, "everyone", [])).toEqual({ remove: ["1", "2"], add: [{ audience: "everyone", viewerUserId: null }] });
    expect(shareChanges(saved, "private", ["amy"])).toEqual({ remove: ["1", "2"], add: [] });
    expect(shareChanges([share("3", "everyone")], "everyone", [])).toEqual({ remove: [], add: [] });
  });
});
