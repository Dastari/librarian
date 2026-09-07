import { describe, expect, it } from "vitest";

import { formatBytes, formatClock, formatRelative, formatRuntime, joinMeta, parseTimestamp, pluralize, sortKey } from "../format";

describe("format", () => {
  it("formats bytes with sensible units", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(8_107_000_000)).toBe("7.6 GB");
    expect(formatBytes(null)).toBe("—");
  });

  it("formats clocks and runtimes", () => {
    expect(formatClock(125)).toBe("2:05");
    expect(formatClock(5400)).toBe("1:30:00");
    expect(formatRuntime(8107)).toBe("2h 15m");
    expect(formatRuntime(135, "minutes")).toBe("2h 15m");
    expect(formatRuntime(45, "minutes")).toBe("45m");
  });

  it("parses ISO strings and unix seconds", () => {
    expect(parseTimestamp("2026-09-05T00:00:00Z")?.getUTCFullYear()).toBe(2026);
    expect(parseTimestamp("1757030400")?.getUTCFullYear()).toBe(2025);
    expect(parseTimestamp(1757030400)?.getUTCFullYear()).toBe(2025);
    expect(parseTimestamp("not a date")).toBeNull();
  });

  it("describes relative time", () => {
    const now = Date.parse("2026-09-05T12:00:00Z");
    expect(formatRelative("2026-09-05T11:59:50Z", now)).toBe("just now");
    expect(formatRelative("2026-09-05T09:00:00Z", now)).toBe("3 hours ago");
  });

  it("joins meta and pluralizes", () => {
    expect(joinMeta(1990, null, "2h 15m", "")).toBe("1990 · 2h 15m");
    expect(pluralize(1, "movie")).toBe("1 movie");
    expect(pluralize(4, "show")).toBe("4 shows");
    expect(sortKey("The Matrix")).toBe("matrix");
  });
});
