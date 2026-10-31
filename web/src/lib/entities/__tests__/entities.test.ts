import type { DataTableState } from "glass-ux/data-table";
import { describe, expect, it } from "vitest";

import { activityVariables } from "@/features/activity/useActivity";
import { downloadsVariables } from "@/features/downloads/useDownloads";
import { issueVariables, runVariables } from "@/features/libraries/scans/useScans";

import { humanize, linkedItem, mediaFileRef } from "../linked";
import { notificationDecision, notificationSubjects, upgradeDetails } from "../notification";
import { issueDetails, scanDuration } from "../scan";
import { ratioLevel, torrentTarget } from "../torrent";

const state = (partial: Partial<DataTableState> = {}): DataTableState => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: {}, sorting: [], filters: [], ...partial }) as DataTableState;
const options = (...values: string[]) => ({ op: "in", values });

describe("linked records", () => {
  it("names a file's item, most specific first", () => {
    expect(linkedItem({ episode: { id: "e1", showId: "s1", season: 1, episode: 3, title: "The Head", show: { id: "s1", name: "Fallout" } } })).toMatchObject({ kind: "episode", label: "Fallout · S01E03", href: "/shows/s1" });
    expect(linkedItem({ movie: { id: "m1", title: "Arrival", year: 2016 } })).toMatchObject({ kind: "movie", meta: "2016", href: "/movies/m1" });
    expect(linkedItem({})).toBeNull();
  });

  it("opens a file in its library's Files view, filtered to its name", () => {
    expect(mediaFileRef({ id: "f1", libraryId: "l1", path: "/media/Movies/Arrival (2016).mkv" })).toMatchObject({ label: "Arrival (2016).mkv", href: "/libraries/l1/files?q=Arrival%20(2016).mkv" });
  });

  it("writes server codes as words", () => {
    expect(humanize("BYTE_IDENTICAL_DUPLICATE")).toBe("Byte identical duplicate");
    expect(humanize("auto_download")).toBe("Auto download");
  });
});

describe("notifications", () => {
  it("lists what a notification is about: item, file, download, library", () => {
    const subjects = notificationSubjects({
      library: { id: "l1", name: "Movies" },
      torrent: { id: "t1", name: "Arrival.2016.1080p" },
      mediaFile: { id: "f1", libraryId: "l1", path: "/m/Arrival.mkv", movie: { id: "m1", title: "Arrival" } },
    });
    expect(subjects.map((subject) => subject.kind)).toEqual(["movie", "mediaFile", "torrent", "library"]);
    expect(subjects[2]!.href).toBe("/downloads?torrent=t1");
  });

  it("reads a decision's state", () => {
    expect(notificationDecision({ actionType: null })).toBeNull();
    expect(notificationDecision({ actionType: "quality_upgrade" })).toMatchObject({ label: "Approve upgrade?" });
    expect(notificationDecision({ actionType: "quality_upgrade", resolvedAt: "x", resolution: "ACCEPTED" })).toMatchObject({ label: "Approved", tone: "success" });
    expect(notificationDecision({ actionType: "quality_upgrade", resolvedAt: "x", resolution: "dismissed" })).toMatchObject({ label: "Dismissed" });
  });

  it("parses an upgrade's resolutions and tolerates bad data", () => {
    expect(upgradeDetails('{"existingResolution":"1920x800","newResolution":"1080p","targetPath":"/a.mkv"}')).toEqual({ from: "1920x800", to: "1080p", sourcePath: undefined, targetPath: "/a.mkv" });
    expect(upgradeDetails("not json")).toBeNull();
  });
});

describe("scans", () => {
  it("formats durations and keeps scalar details", () => {
    expect(scanDuration({ startedAt: "2026-10-01T00:00:00Z", finishedAt: "2026-10-01T00:00:38Z" })).toBe("38 s");
    expect(scanDuration({ startedAt: "2026-10-01T00:00:00Z", finishedAt: "2026-10-01T00:12:00Z" })).toBe("12 min");
    expect(scanDuration({ startedAt: null, finishedAt: null })).toBeNull();
    expect(issueDetails('{"keeperPath":"/a","size":3,"nested":{"x":1}}')).toEqual({ keeperPath: "/a", size: 3 });
  });
});

describe("torrents", () => {
  it("targets the episode, the season pack or the movie", () => {
    expect(torrentTarget({ show: { id: "s1", name: "Silo" }, season: 2 })).toMatchObject({ kind: "show", label: "Silo · Season 2" });
    expect(torrentTarget({})).toBeNull();
    expect(ratioLevel(0.4, null)).toBe("short");
    expect(ratioLevel(0.6, null)).toBe("close");
    expect(ratioLevel(2, 2)).toBe("met");
  });
});

describe("table state as server queries", () => {
  it("downloads: preset, filters, search and sorting", () => {
    const { where, orderBy } = downloadsVariables(state({ filters: [{ id: "import", value: options("waiting", "failed") }], search: "silo", sorting: [{ id: "size", desc: true }] }), "seeding");
    expect(where).toEqual({ and: [{ state: { eq: "seeding" } }, { or: [{ postProcessStatus: { inList: ["failed"] } }, { postProcessStatus: { isNull: true } }] }, { name: { contains: "silo" } }] });
    expect(orderBy).toEqual([{ totalBytes: "DESC" }]);
  });

  it("activity: unread preset, level filter, message search", () => {
    const { where, orderBy } = activityVariables(state({ filters: [{ id: "level", value: options("ERROR") }], search: "conflict" }), "unread");
    expect(where).toEqual({ and: [{ readAt: { isNull: true } }, { notificationType: { inList: ["ERROR"] } }, { or: [{ title: { contains: "conflict" } }, { message: { contains: "conflict" } }, { category: { contains: "conflict" } }] }] });
    expect(orderBy).toEqual([{ createdAt: "DESC" }]);
  });

  it("scan issues: library, view, one run, filters and sorting", () => {
    const { where, orderBy } = issueVariables("l1", state({ filters: [{ id: "severity", value: options("WARNING") }], sorting: [{ id: "seen", desc: true }] }), "open", "run1");
    expect(where).toEqual({ and: [{ libraryId: { eq: "l1" } }, { resolvedAt: { isNull: true } }, { scanRunId: { eq: "run1" } }, { severity: { inList: ["WARNING"] } }] });
    expect(orderBy).toEqual([{ occurrenceCount: "DESC" }]);
  });

  it("scan runs: status filter, newest first by default", () => {
    const { where, orderBy } = runVariables("l1", state({ filters: [{ id: "status", value: options("FAILED") }] }));
    expect(where).toEqual({ and: [{ libraryId: { eq: "l1" } }, { status: { inList: ["FAILED"] } }] });
    expect(orderBy).toEqual([{ createdAt: "DESC" }]);
  });

  it("refuses filters a table does not declare", () => {
    expect(() => activityVariables(state({ filters: [{ id: "nope", value: options("x") }] }), "all")).toThrow(/Unsupported/);
  });
});
