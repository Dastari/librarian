import { describe, expect, it } from "vitest";

import { planBadgeFor } from "../TorrentFiles";

describe("the match plan's badge for a file", () => {
  it("labels music files by their album, never as another book", () => {
    expect(planBadgeFor("album")).toEqual({ label: "On album", tone: "info" });
    expect(planBadgeFor("new")).toEqual({ label: "New album", tone: "neutral" });
  });

  it("keeps the audiobook labels", () => {
    expect(planBadgeFor("match").label).toBe("Best match");
    expect(planBadgeFor("review").label).toBe("Needs review");
    expect(planBadgeFor("other").label).toBe("Other book");
  });

  it("falls back to No match for a status it does not know", () => {
    expect(planBadgeFor("surprise").label).toBe("No match");
  });
});
