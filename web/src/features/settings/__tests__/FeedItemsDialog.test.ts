import { describe, expect, it } from "vitest";

import { itemQuality } from "../FeedItemsDialog";

describe("itemQuality", () => {
  it("reads what a title says as one short line", () => {
    expect(itemQuality({ parsedResolution: "1080p", parsedSource: "bluray", parsedCodec: "h264", parsedHdr: null, parsedGroup: "GRP", parsedFormat: null, parsedBitrate: null })).toBe("1080p · bluray · h264 · GRP");
  });

  it("includes the format and bitrate of music", () => {
    expect(itemQuality({ parsedResolution: null, parsedSource: null, parsedCodec: null, parsedHdr: null, parsedGroup: null, parsedFormat: "FLAC", parsedBitrate: "24BIT" })).toBe("FLAC · 24BIT");
  });

  it("is empty when the title says nothing", () => {
    expect(itemQuality({ parsedResolution: null, parsedSource: null, parsedCodec: null, parsedHdr: null, parsedGroup: null, parsedFormat: null, parsedBitrate: null })).toBe("");
  });
});
