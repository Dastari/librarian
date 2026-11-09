import { describe, expect, it } from "vitest";

import { folderPath, folderUri, shareRootOf } from "../FolderPickerDialog";

describe("server folder addresses", () => {
  it("round-trips paths with spaces and punctuation", () => {
    for (const path of ["/data/media/Audio Books", "/data/Guns N’ Roses/#1 (1991)", "/"]) {
      expect(folderPath(folderUri(path))).toBe(path);
    }
  });

  it("keeps the location's scheme", () => {
    expect(folderUri("/data/media")).toBe("server:///data/media");
    expect(folderUri("data")).toBe("server:///data");
  });

  it("addresses a folder on a share as nested segments, and back", () => {
    const path = "smb://10.0.1.190/media/Movies/Film (2020)";
    expect(folderUri(path)).toBe("server:///smb%3A/10.0.1.190/media/Movies/Film%20(2020)");
    expect(folderPath(folderUri(path))).toBe(path);
    expect(folderPath(folderUri("smb://nas/media"))).toBe("smb://nas/media");
  });

  it("finds the share a path is on", () => {
    expect(shareRootOf("smb://nas/media/Movies")).toBe("smb://nas/media");
    expect(shareRootOf("smb://nas/media")).toBe("smb://nas/media");
    expect(shareRootOf("/data/media")).toBeNull();
    expect(shareRootOf("smb://nas")).toBeNull();
    expect(shareRootOf(undefined)).toBeNull();
  });
});
