import { describe, expect, it } from "vitest";

import { parseStatus } from "../AutoDownloadStatus";

describe("parseStatus", () => {
  it("reads the passes the scheduler recorded", () => {
    const status = parseStatus(JSON.stringify({ sweep: { at: "2026-11-08T12:00:00Z", searched: 4, grabbed: 1 } }));
    expect(status.sweep?.searched).toBe(4);
  });

  it("treats a missing, damaged or non-object value as nothing having run", () => {
    expect(parseStatus(undefined)).toEqual({});
    expect(parseStatus("")).toEqual({});
    expect(parseStatus("{not json")).toEqual({});
    expect(parseStatus("[1,2]")).toEqual({});
    expect(parseStatus("12")).toEqual({});
  });
});
