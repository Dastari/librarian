import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  formatBytes,
  formatDuration,
  formatEta,
  formatRelativeTime,
  formatSpeed,
  previewNamingPattern,
  sanitizeError,
} from "../../src/lib/format";

describe("format utilities", () => {
  beforeEach(() => {
    vi.useRealTimers();
  });

  it("formats bytes, speeds, ETA values, and playback durations", () => {
    expect(formatBytes(undefined)).toBe("0 B");
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(1_610_612_736)).toBe("1.5 GB");

    expect(formatSpeed(1536)).toBe("1.5 KB/s");
    expect(formatEta(undefined)).toBe("—");
    expect(formatEta(59)).toBe("59s");
    expect(formatEta(125)).toBe("2m");
    expect(formatEta(7_500)).toBe("2h 5m");
    expect(formatEta(176_400)).toBe("2d 1h");

    expect(formatDuration(undefined)).toBe("0:00");
    expect(formatDuration(65)).toBe("1:05");
    expect(formatDuration(3_665)).toBe("1:01:05");
  });

  it("formats relative time around the current clock", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-05T12:00:00Z"));

    expect(formatRelativeTime(undefined)).toBe("Never");
    expect(formatRelativeTime("2026-05-05T12:00:00Z")).toBe("just now");
    expect(formatRelativeTime("2026-05-05T11:58:00Z")).toBe("2m ago");
    expect(formatRelativeTime("2026-05-05T09:00:00Z")).toBe("3h ago");
    expect(formatRelativeTime("2026-05-03T12:00:00Z")).toBe("2d ago");
    expect(formatRelativeTime("2026-05-05T12:03:00Z")).toBe("in 3m");
  });

  it("sanitizes backend errors before display", () => {
    expect(sanitizeError(null)).toBe("Unknown error");
    expect(sanitizeError("plain error")).toBe("plain error");
    expect(sanitizeError(new Error("<!DOCTYPE html><html></html>"))).toBe(
      "Failed to connect to server. Please check that the backend is running.",
    );
    expect(sanitizeError("x".repeat(205))).toBe(`${"x".repeat(200)}...`);
  });

  it("previews naming patterns with library-specific sample values", () => {
    expect(previewNamingPattern("{title} ({year})/{title}.{ext}", "movies")).toBe(
      "The Matrix (2008)/The Matrix.mkv",
    );
    expect(previewNamingPattern("{artist}/{album}/{track:02} - {title}.{ext}", "music")).toBe(
      "Pink Floyd/The Dark Side of the Moon/03 - Time.flac",
    );
    expect(previewNamingPattern("{author}/{series}/{title}.{ext}", "audiobooks")).toBe(
      "Brandon Sanderson/The Stormlight Archive/The Way of Kings.m4b",
    );
  });
});

