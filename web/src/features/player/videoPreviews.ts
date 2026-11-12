import type { MediaPreviewFrame, MediaStoryboard, MediaThumbnail } from "glass-ux/media-player";

/** The server's storyboard description (`MediaFile.storyboard`, design.md Q122). */
export interface StoryboardInfo {
  key: string;
  interval: number;
  tileWidth: number;
  tileHeight: number;
  columns: number;
  rows: number;
  frameCount: number;
  sheetCount: number;
}

/** The picture inside baked-in black bars (`MediaFile.pictureCrop`, design.md Q123). */
export interface PictureCropInfo {
  left: number;
  top: number;
  width: number;
  height: number;
  frameWidth: number;
  frameHeight: number;
}

export function storyboardSheetUrl(mediaFileId: string, key: string, sheet: number): string {
  return `/api/media/${encodeURIComponent(mediaFileId)}/storyboard/${encodeURIComponent(key)}/${sheet}.jpg`;
}

function usable(board: StoryboardInfo | null | undefined): board is StoryboardInfo {
  return Boolean(board && board.interval > 0 && board.frameCount > 0 && board.columns > 0 && board.rows > 0 && board.tileWidth > 0 && board.tileHeight > 0);
}

/** The tile for a moment: the same maths as the server's `StoryboardLayout::tile_for`. */
export function storyboardFrame(mediaFileId: string, board: StoryboardInfo, seconds: number): MediaPreviewFrame {
  const index = Number.isFinite(seconds) && seconds > 0 ? Math.min(Math.floor(seconds / board.interval), board.frameCount - 1) : 0;
  const perSheet = board.columns * board.rows;
  const within = index % perSheet;
  return {
    src: storyboardSheetUrl(mediaFileId, board.key, Math.floor(index / perSheet)),
    x: (within % board.columns) * board.tileWidth,
    y: Math.floor(within / board.columns) * board.tileHeight,
    width: board.tileWidth,
    height: board.tileHeight,
  };
}

/**
 * What to hand Glass's player. Its `storyboard` describes one sheet, so a file whose previews fit
 * one sheet gets that; a longer file gets a `thumbnail` callback that also picks the sheet.
 */
export function storyboardProps(mediaFileId: string, board: StoryboardInfo | null | undefined): { storyboard?: MediaStoryboard; thumbnail?: MediaThumbnail } {
  if (!usable(board)) return {};
  if (board.sheetCount <= 1) {
    return { storyboard: { src: storyboardSheetUrl(mediaFileId, board.key, 0), tileWidth: board.tileWidth, tileHeight: board.tileHeight, columns: board.columns, interval: board.interval } };
  }
  return { thumbnail: (seconds) => storyboardFrame(mediaFileId, board, seconds) };
}

export interface FillTransform {
  scale: number;
  /** Shift after scaling, in CSS pixels of the box. */
  x: number;
  y: number;
}

const NO_FILL: FillTransform = { scale: 1, x: 0, y: 0 };

/**
 * How far to zoom a video drawn with `object-fit: contain` in a `boxWidth` x `boxHeight` box so
 * the picture inside its black bars fills as much of the box as it can without being cut.
 *
 * The crop is taken as fractions of the frame it was measured in, so it holds for whatever
 * size the element is decoding (direct play at source size, or an HLS rendition that was scaled).
 * A box no wider (letterbox) or taller (pillarbox) than the picture needs no zoom: scale 1.
 */
export function fillTransform(videoWidth: number, videoHeight: number, crop: PictureCropInfo, boxWidth: number, boxHeight: number): FillTransform {
  if (!(videoWidth > 0 && videoHeight > 0 && boxWidth > 0 && boxHeight > 0 && crop.frameWidth > 0 && crop.frameHeight > 0 && crop.width > 0 && crop.height > 0)) return NO_FILL;
  const left = crop.left / crop.frameWidth;
  const top = crop.top / crop.frameHeight;
  const width = Math.min(1, crop.width / crop.frameWidth);
  const height = Math.min(1, crop.height / crop.frameHeight);
  // The size the element draws the whole frame at.
  const fitted = Math.min(boxWidth / videoWidth, boxHeight / videoHeight);
  const drawnWidth = videoWidth * fitted;
  const drawnHeight = videoHeight * fitted;
  // Grow until the picture meets the box on one side.
  const scale = Math.max(1, Math.min(boxWidth / (drawnWidth * width), boxHeight / (drawnHeight * height)));
  if (scale < 1.005) return NO_FILL;
  // Bring the picture's centre to the box's centre (the element scales about its own centre).
  const offsetX = (left + width / 2 - 0.5) * drawnWidth;
  const offsetY = (top + height / 2 - 0.5) * drawnHeight;
  return { scale, x: -offsetX * scale, y: -offsetY * scale };
}

export function fillTransformCss(transform: FillTransform): string {
  if (transform.scale === 1 && transform.x === 0 && transform.y === 0) return "";
  return `translate(${transform.x.toFixed(2)}px, ${transform.y.toFixed(2)}px) scale(${transform.scale.toFixed(4)})`;
}

/** The per-device preference key for the picture mode (`prefs.ts`). */
export const PICTURE_MODE_PREF = "player.pictureMode";
export type PictureMode = "fit" | "fill";
