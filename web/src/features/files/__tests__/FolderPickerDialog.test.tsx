import { describe, expect, it } from "vitest";

import { folderPath, folderUri } from "../FolderPickerDialog";

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
});
