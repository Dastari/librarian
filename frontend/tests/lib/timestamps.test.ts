import { describe, expect, it } from "vitest";
import {
  parseTimestamp,
  formatDate,
  formatDateTime,
  formatRelativeTime,
} from "../../src/lib/format";
describe("entity timestamps", () => {
  it("reads Unix seconds and ISO dates as the same instant", () => {
    expect(parseTimestamp("1785390864")?.toISOString()).toBe(
      "2026-07-30T05:54:24.000Z",
    );
    expect(parseTimestamp("2026-07-30T07:54:24+02:00")?.getTime()).toBe(
      parseTimestamp("1785390864")?.getTime(),
    );
    expect(parseTimestamp("0")?.getTime()).toBe(0);
  });
  it("does not show Invalid Date for missing or corrupt timestamps", () => {
    for (const value of [undefined, "", "bad date", "9223372036854775807"]) {
      expect(parseTimestamp(value)).toBeNull();
      expect(formatDate(value, "Unknown")).toBe("Unknown");
      expect(formatDateTime(value)).toBe("Never");
      expect(formatRelativeTime(value)).toBe("Never");
    }
  });
});
