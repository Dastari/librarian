import { describe, expect, it } from "vitest";

import { fillTransform, fillTransformCss, storyboardFrame, storyboardProps, storyboardSheetUrl, type PictureCropInfo, type StoryboardInfo } from "../videoPreviews";

// A two-hour film: 360 tiles every 20 s on four 10x10 sheets (backend `storyboard::plan`).
const film: StoryboardInfo = { key: "0123456789abcdef", interval: 20, tileWidth: 240, tileHeight: 136, columns: 10, rows: 10, frameCount: 360, sheetCount: 4 };
// Fallout S01E01: a 1920x804 picture in a 1920x1080 frame.
const letterbox: PictureCropInfo = { left: 0, top: 138, width: 1920, height: 804, frameWidth: 1920, frameHeight: 1080 };
const pillarbox: PictureCropInfo = { left: 240, top: 0, width: 1440, height: 1080, frameWidth: 1920, frameHeight: 1080 };

describe("storyboard previews", () => {
  it("builds sheet URLs under the media file", () => {
    expect(storyboardSheetUrl("file 1", film.key, 2)).toBe("/api/media/file%201/storyboard/0123456789abcdef/2.jpg");
  });

  it("finds a moment's tile across sheets, as the server lays them out", () => {
    expect(storyboardFrame("f", film, 0)).toEqual({ src: storyboardSheetUrl("f", film.key, 0), x: 0, y: 0, width: 240, height: 136 });
    expect(storyboardFrame("f", film, 25 * 60)).toMatchObject({ src: storyboardSheetUrl("f", film.key, 0), x: 5 * 240, y: 7 * 136 });
    expect(storyboardFrame("f", film, 2000)).toMatchObject({ src: storyboardSheetUrl("f", film.key, 1), x: 0, y: 0 });
    expect(storyboardFrame("f", film, 99_999)).toMatchObject({ src: storyboardSheetUrl("f", film.key, 3), x: 9 * 240, y: 5 * 136 });
    expect(storyboardFrame("f", film, Number.NaN)).toMatchObject({ x: 0, y: 0 });
  });

  it("hands Glass a storyboard for one sheet and a thumbnail callback for several", () => {
    const short = { ...film, frameCount: 40, sheetCount: 1, interval: 10 };
    expect(storyboardProps("f", short)).toEqual({ storyboard: { src: storyboardSheetUrl("f", film.key, 0), tileWidth: 240, tileHeight: 136, columns: 10, interval: 10 } });
    const long = storyboardProps("f", film);
    expect(long.storyboard).toBeUndefined();
    expect(long.thumbnail?.(2000)).toMatchObject({ src: storyboardSheetUrl("f", film.key, 1) });
  });

  it("offers nothing without a usable storyboard", () => {
    expect(storyboardProps("f", null)).toEqual({});
    expect(storyboardProps("f", { ...film, interval: 0 })).toEqual({});
  });
});

describe("fill mode", () => {
  it("zooms a letterboxed picture to fill a wider window, keeping it centred", () => {
    const zoom = fillTransform(1920, 1080, letterbox, 2560, 1080);
    expect(zoom.scale).toBeCloseTo(2560 / 1920, 4);
    expect(zoom.x).toBeCloseTo(0, 4);
    expect(zoom.y).toBeCloseTo(0, 4);
    // A 32:9 screen: the picture's height limits the zoom.
    expect(fillTransform(1920, 1080, letterbox, 3840, 1080).scale).toBeCloseTo(1080 / 804, 4);
  });

  it("leaves a 16:9 screen alone, as the picture already meets its sides", () => {
    expect(fillTransform(1920, 1080, letterbox, 1920, 1080)).toEqual({ scale: 1, x: 0, y: 0 });
    // A portrait phone: narrower than the frame, nothing to fill.
    expect(fillTransform(1920, 1080, letterbox, 390, 844)).toEqual({ scale: 1, x: 0, y: 0 });
    expect(fillTransformCss({ scale: 1, x: 0, y: 0 })).toBe("");
  });

  it("zooms a pillarboxed picture on a taller screen", () => {
    const zoom = fillTransform(1920, 1080, pillarbox, 1024, 768);
    expect(zoom.scale).toBeCloseTo(4 / 3, 4);
    expect(zoom.x).toBeCloseTo(0, 4);
  });

  it("holds for an HLS rendition smaller than the source", () => {
    // The crop is a share of the frame, so a 1280x720 rendition zooms the same.
    expect(fillTransform(1280, 720, letterbox, 2560, 1080).scale).toBeCloseTo(fillTransform(1920, 1080, letterbox, 2560, 1080).scale, 6);
  });

  it("re-centres a picture that sits off centre", () => {
    const offCentre: PictureCropInfo = { left: 0, top: 100, width: 1920, height: 804, frameWidth: 1920, frameHeight: 1080 };
    const zoom = fillTransform(1920, 1080, offCentre, 2560, 1080);
    // Its centre is 38 px above the frame's; scaled by 4/3 that moves it down 50.67 px.
    expect(zoom.y).toBeCloseTo(38 * (2560 / 1920), 2);
    expect(fillTransformCss(zoom)).toBe("translate(0.00px, 50.67px) scale(1.3333)");
  });

  it("does nothing until the element knows its size", () => {
    expect(fillTransform(0, 0, letterbox, 2560, 1080)).toEqual({ scale: 1, x: 0, y: 0 });
  });
});
