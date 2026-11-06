import { describe, expect, it } from "vitest";

import { contentStatusMeta, type ContentStatusMap } from "../useContentStatuses";

const statuses = (entries: Array<[string, "DOWNLOADING" | "WANTED"]>, progress: Array<[string, number]> = []) =>
  Object.assign(new Map(entries), { progress: new Map(progress) }) as ContentStatusMap;

describe("the status shown for a title", () => {
  it("says how far along a running download is", () => {
    expect(contentStatusMeta(statuses([["a", "DOWNLOADING"]], [["a", 0.369]]), "a").label).toBe("Downloading 36%");
  });

  it("doesn't round up to done before it is", () => {
    expect(contentStatusMeta(statuses([["a", "DOWNLOADING"]], [["a", 0.999]]), "a").label).toBe("Downloading 99%");
  });

  it("just says downloading when the amount isn't known", () => {
    expect(contentStatusMeta(statuses([["a", "DOWNLOADING"]]), "a").label).toBe("Downloading");
  });

  it("leaves other statuses alone", () => {
    expect(contentStatusMeta(statuses([["a", "WANTED"]], [["a", 0.5]]), "a").label).not.toContain("%");
  });
});
