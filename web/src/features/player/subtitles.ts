/**
 * What the player offers from a file's `Subtitle` rows (docs/design.md Q118–Q120): text tracks
 * the server converts to WebVTT (embedded, sidecar and downloaded alike), and picture subtitles
 * (PGS, VobSub) it can only burn into the video. Labels carry the language and the flags a
 * person chooses by: Forced, SDH, and where the track came from.
 */
import type { MediaTextTrack } from "glass-ux/media-player";

import { mediaSubtitleUrl } from "@/lib/api/urls";
import { displayLanguage, sameLanguage, toBcp47 } from "@/lib/languages";
import { readPref, writePref } from "@/lib/prefs";

export interface SubtitleRow {
  id: string;
  sourceType: string;
  codec?: string | null;
  title?: string | null;
  language?: string | null;
  isDefault?: boolean | null;
  isForced?: boolean | null;
  isHearingImpaired?: boolean | null;
}

const TEXT_CODECS = new Set(["subrip", "srt", "webvtt", "vtt", "ass", "ssa", "mov_text", "text", "microdvd"]);
const BITMAP_CODECS = new Set(["hdmv_pgs_subtitle", "pgssub", "pgs", "dvd_subtitle", "dvdsub", "vobsub", "dvb_subtitle", "dvbsub", "xsub"]);

export const isTextSubtitle = (row: Pick<SubtitleRow, "codec">) => TEXT_CODECS.has(row.codec?.toLowerCase() ?? "");
export const isBitmapSubtitle = (row: Pick<SubtitleRow, "codec">) => BITMAP_CODECS.has(row.codec?.toLowerCase() ?? "");

const SOURCE_LABEL: Record<string, string> = { sidecar: "File", external: "File", downloaded: "Downloaded" };

/** `English · Forced · SDH · Downloaded`, or the row's title when it names no language. */
export function subtitleLabel(row: SubtitleRow): string {
  const language = displayLanguage(row.language);
  const parts: string[] = [language ?? row.title?.trim() ?? "Unknown language"];
  // A title that says more than the language (Commentary, Signs & Songs) is worth showing.
  const title = row.title?.trim();
  if (language && title && row.sourceType !== "downloaded" && title.toLowerCase() !== language.toLowerCase() && title.length <= 40) parts.push(title);
  if (row.isForced) parts.push("Forced");
  if (row.isHearingImpaired) parts.push("SDH");
  const source = SOURCE_LABEL[row.sourceType.toLowerCase()];
  if (source) parts.push(source);
  return parts.join(" · ");
}

/** Text tracks for Glass's captions menu, with the timing offset applied by the server. */
export function subtitleTracks(mediaFileId: string, rows: readonly SubtitleRow[], offsetMs = 0): MediaTextTrack[] {
  return rows
    .filter(isTextSubtitle)
    .map((row) => {
      const base = mediaSubtitleUrl(mediaFileId, row.id);
      return {
        id: row.id,
        src: offsetMs ? `${base}?offsetMs=${Math.round(offsetMs)}` : base,
        label: subtitleLabel(row),
        language: toBcp47(row.language) ?? "und",
        kind: row.isHearingImpaired ? "captions" as const : "subtitles" as const,
      };
    });
}

/** What is remembered for a show: a language and its flags, or that subtitles were off. */
export interface SubtitleChoice {
  off: boolean;
  language?: string | null;
  forced?: boolean;
  hearingImpaired?: boolean;
  /** A picture subtitle, burned in. */
  burned?: boolean;
}

/** The choice a row stands for. */
export function choiceOf(row: SubtitleRow | undefined, burned = false): SubtitleChoice {
  if (!row) return { off: true };
  return { off: false, language: row.language ?? null, forced: Boolean(row.isForced), hearingImpaired: Boolean(row.isHearingImpaired), burned };
}

/**
 * The row a remembered choice picks among a file's subtitles: the same language, then the same
 * flags, then text over pictures (pictures only when that was the choice), then the file's
 * default. `undefined` when nothing matches or subtitles were off.
 */
export function pickSubtitle(rows: readonly SubtitleRow[], choice: SubtitleChoice | null): SubtitleRow | undefined {
  if (!choice || choice.off) return undefined;
  const usable = rows.filter((row) => isTextSubtitle(row) || (choice.burned && isBitmapSubtitle(row)));
  const candidates = usable.filter((row) => (choice.language ? sameLanguage(row.language, choice.language) : !row.language));
  if (!candidates.length) return undefined;
  const score = (row: SubtitleRow) =>
    (Boolean(row.isForced) === Boolean(choice.forced) ? 0 : 4) +
    (Boolean(row.isHearingImpaired) === Boolean(choice.hearingImpaired) ? 0 : 2) +
    (isTextSubtitle(row) === !choice.burned ? 0 : 1) +
    (row.isDefault ? 0 : 0.5);
  return [...candidates].sort((a, b) => score(a) - score(b))[0];
}

const prefKey = (userId: string, showId: string) => `subtitles.show.${userId}.${showId}`;

/** The choice remembered for a show, on this device. */
export function readShowChoice(userId: string | undefined, showId: string | undefined): SubtitleChoice | null {
  if (!userId || !showId) return null;
  try {
    return readPref<SubtitleChoice | null>(prefKey(userId, showId), null);
  } catch {
    return null;
  }
}

export function writeShowChoice(userId: string | undefined, showId: string | undefined, choice: SubtitleChoice): void {
  if (!userId || !showId) return;
  try {
    writePref(prefKey(userId, showId), choice);
  } catch {
    // Storage can be full or blocked; the choice is a convenience.
  }
}

/** Offset steps a remote can reach: a quarter of a second each, ten minutes at most. */
export const OFFSET_STEP_MS = 250;
export const MAX_OFFSET_MS = 10 * 60 * 1000;
export const clampOffset = (value: number) => Math.max(-MAX_OFFSET_MS, Math.min(MAX_OFFSET_MS, Math.round(value)));

/** `+0.75 s`, `−1.5 s`, `In sync`. */
export function formatOffset(offsetMs: number): string {
  if (!offsetMs) return "In sync";
  const seconds = Math.abs(offsetMs) / 1000;
  const text = Number.isInteger(seconds) ? `${seconds}` : seconds.toFixed(seconds * 100 % 10 === 0 ? 1 : 2);
  return `${offsetMs > 0 ? "+" : "−"}${text} s`;
}
