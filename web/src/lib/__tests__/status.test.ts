import { describe, expect, it } from "vitest";

import { notificationType, qualityStatus, scanStatus, statusMeta, torrentState } from "../status";

describe("status vocab", () => {
  it("maps every content status to a label and tone", () => {
    expect(statusMeta("AVAILABLE").tone).toBe("success");
    expect(statusMeta("WANTED").label).toBe("Wanted");
    expect(statusMeta(undefined).label).toBe("Missing");
  });

  it("is forgiving about unknown backend strings", () => {
    expect(torrentState("weird").label).toBe("weird");
    expect(torrentState("Seeding").tone).toBe("success");
    expect(scanStatus("COMPLETED_WITH_ISSUES").tone).toBe("warning");
    expect(notificationType(null).label).toBe("Info");
    expect(qualityStatus("SUBOPTIMAL").label).toBe("Below target");
  });
});
