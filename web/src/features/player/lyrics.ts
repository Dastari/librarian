/**
 * Lyrics as the server stores them (docs/design.md Q121): synced lyrics normalised to one
 * `[mm:ss.cc]text` line per time, sorted, so reading them is a split; plain lyrics as text.
 */
export interface LyricLine {
  /** Seconds into the track. */
  start: number;
  text: string;
}

const STAMP = /^\[(\d+):(\d{1,2})(?:[.:](\d{1,3}))?\](.*)$/;

/** Timed lines of stored LRC, sorted. Lines without a time are left out. */
export function parseLrc(text: string | null | undefined): LyricLine[] {
  if (!text) return [];
  const lines: LyricLine[] = [];
  for (const raw of text.split(/\r?\n/)) {
    const match = STAMP.exec(raw.trim());
    if (!match) continue;
    const [, minutes, seconds, fraction = "", words = ""] = match;
    const ms = fraction ? Number(fraction.padEnd(3, "0")) : 0;
    lines.push({ start: Number(minutes) * 60 + Number(seconds) + ms / 1000, text: words.trim() });
  }
  return lines.sort((a, b) => a.start - b.start);
}

/** The line being sung at `position` (seconds), or -1 before the first. */
export function activeLine(lines: readonly LyricLine[], position: number): number {
  let low = 0;
  let high = lines.length - 1;
  let found = -1;
  while (low <= high) {
    const middle = (low + high) >> 1;
    if (lines[middle]!.start <= position + 0.05) {
      found = middle;
      low = middle + 1;
    } else {
      high = middle - 1;
    }
  }
  return found;
}

/** Plain lyrics as stanzas: blank lines separate them. */
export function stanzas(text: string | null | undefined): string[][] {
  if (!text) return [];
  return text
    .split(/\r?\n\s*\r?\n/)
    .map((stanza) => stanza.split(/\r?\n/).map((line) => line.trim()).filter(Boolean))
    .filter((stanza) => stanza.length > 0);
}

const SOURCE_NAME: Record<string, string> = { lrclib: "LRCLIB", embedded: "the file's tags", sidecar: "an .lrc file beside the track" };

/** "Lyrics from LRCLIB". */
export function lyricsCredit(source: string | null | undefined): string | undefined {
  const name = source ? SOURCE_NAME[source] : undefined;
  return name ? `Lyrics from ${name}` : undefined;
}
