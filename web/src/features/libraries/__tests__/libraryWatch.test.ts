import { describe, expect, it } from "vitest";

import { libraryWatch } from "../libraryWatch";

const share = { path: "smb://nas/media/TV Shows", watchForChanges: true };
const NOW = Date.parse("2026-11-09T10:00:00Z");

describe("a library's folder watch", () => {
  it("is off when the library isn't watched, whatever was last said", () => {
    expect(libraryWatch({ ...share, watchForChanges: false, watchStatus: "watching" })).toMatchObject({ id: "off", tone: "neutral" });
  });

  it("reads the backend's states", () => {
    expect(libraryWatch({ ...share, watchStatus: "watching" })).toMatchObject({ label: "Watching for changes", tone: "success" });
    expect(libraryWatch({ ...share, watchStatus: null })).toMatchObject({ id: "starting", tone: "info" });
    expect(libraryWatch({ ...share, watchStatus: "failed", watchDetail: "No such folder" })).toMatchObject({ tone: "danger", detail: "No such folder" });
  });

  it("names a share that can't be reached, and since when", () => {
    const watch = libraryWatch({ ...share, watchStatus: "unavailable", watchStatusSince: "2026-11-09T09:55:00Z" }, NOW);
    expect(watch).toMatchObject({ label: "Share not reachable, retrying", tone: "warning" });
    expect(watch.detail).toMatch(/^Unreachable since .+\. Changes made meanwhile are caught up when it's back\.$/);
    expect(libraryWatch({ path: "/data/media/TV", watchForChanges: true, watchStatus: "unavailable" }).label).toBe("Folder not reachable, retrying");
  });
});
