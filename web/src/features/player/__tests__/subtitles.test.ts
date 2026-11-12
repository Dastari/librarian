import { describe, expect, it } from "vitest";

import { choiceOf, formatOffset, pickSubtitle, subtitleLabel, subtitleTracks, type SubtitleRow } from "../subtitles";

const row = (id: string, language: string | null, extra: Partial<SubtitleRow> = {}): SubtitleRow => ({ id, sourceType: "embedded", codec: "subrip", language, ...extra });

describe("subtitles", () => {
  it("labels tracks by language, flags and where they came from", () => {
    expect(subtitleLabel(row("a", "eng"))).toBe("English");
    expect(subtitleLabel(row("b", "eng", { isForced: true, sourceType: "sidecar" }))).toBe("English · Forced · File");
    expect(subtitleLabel(row("c", "fre", { isHearingImpaired: true, sourceType: "downloaded", title: "Film.2020.1080p" }))).toBe("French · SDH · Downloaded");
    expect(subtitleLabel(row("d", "eng", { title: "Commentary" }))).toBe("English · Commentary");
    expect(subtitleLabel(row("e", null, { title: "Signs" }))).toBe("Signs");
  });

  it("asks the server for shifted cues and marks SDH tracks as captions", () => {
    const tracks = subtitleTracks("f", [row("a", "ger", { isHearingImpaired: true }), row("p", "eng", { codec: "hdmv_pgs_subtitle" })], -1500);
    expect(tracks).toEqual([{ id: "a", src: "/api/media/f/subtitles/a.vtt?offsetMs=-1500", label: "German · SDH", language: "de", kind: "captions" }]);
  });

  it("picks the remembered language and flags, and pictures only when they were chosen", () => {
    const rows = [
      row("en-forced", "eng", { isForced: true }),
      row("en", "en"),
      row("en-sdh", "eng", { isHearingImpaired: true, sourceType: "downloaded" }),
      row("fr-pgs", "fre", { codec: "hdmv_pgs_subtitle" }),
    ];
    expect(pickSubtitle(rows, { off: false, language: "eng" })?.id).toBe("en");
    expect(pickSubtitle(rows, { off: false, language: "en", hearingImpaired: true })?.id).toBe("en-sdh");
    expect(pickSubtitle(rows, { off: false, language: "eng", forced: true })?.id).toBe("en-forced");
    expect(pickSubtitle(rows, { off: false, language: "fre" })).toBeUndefined();
    expect(pickSubtitle(rows, { off: false, language: "fre", burned: true })?.id).toBe("fr-pgs");
    expect(pickSubtitle(rows, { off: true })).toBeUndefined();
    expect(choiceOf(rows[2])).toEqual({ off: false, language: "eng", forced: false, hearingImpaired: true, burned: false });
    expect(choiceOf(undefined)).toEqual({ off: true });
  });

  it("formats offsets", () => {
    expect(formatOffset(0)).toBe("In sync");
    expect(formatOffset(250)).toBe("+0.25 s");
    expect(formatOffset(-1500)).toBe("−1.5 s");
    expect(formatOffset(2000)).toBe("+2 s");
  });
});
