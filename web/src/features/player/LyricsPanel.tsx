import { useMutation } from "@apollo/client/react";
import { Button } from "glass-ux/button";
import { Spinner } from "glass-ux/spinner";
import { useEffect, useMemo, useRef, useState } from "react";

import { FetchLyricsDocument, type LyricFieldsFragment } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";
import { useInputMode } from "@/lib/input-mode";
import { cn } from "@/lib/utils";

import { activeLine, lyricsCredit, parseLrc, stanzas } from "./lyrics";

interface LyricsPanelProps {
  mediaFileId: string;
  /** Seconds into the track. */
  position: number;
  onSeek: (seconds: number) => void;
  /** Fill the column beside the artwork (the drawer); otherwise a fixed-height box (the sheet). */
  fill?: boolean;
}

/** After the person scrolls, the lyrics stay where they put them for this long. */
const MANUAL_SCROLL_MS = 4000;

/**
 * The playing track's lyrics (Q121). Synced lyrics highlight the line being sung and keep it in
 * view; pressing a line (or choosing it with a remote) seeks there. Plain lyrics are shown as
 * stanzas. The server finds them once (an `.lrc` beside the file, its tags, then LRCLIB) and
 * caches them; "Look again" asks it to look afresh.
 */
export function LyricsPanel({ mediaFileId, position, onSeek, fill = false }: LyricsPanelProps) {
  const [fetchLyrics] = useMutation(FetchLyricsDocument);
  const [state, setState] = useState<{ fileId: string; lyric: LyricFieldsFragment | null; error: string | null; loading: boolean }>({ fileId: mediaFileId, lyric: null, error: null, loading: true });
  const tv = useInputMode().mode === "tv";

  const load = (refresh: boolean) => {
    setState({ fileId: mediaFileId, lyric: null, error: null, loading: true });
    let cancelled = false;
    void fetchLyrics({ variables: { mediaFileId, refresh } })
      .then(({ data }) => {
        if (cancelled) return;
        const payload = data?.fetchLyrics;
        setState({ fileId: mediaFileId, lyric: payload?.lyric ?? null, error: payload?.success ? null : payload?.error ?? "Lyrics could not be loaded", loading: false });
      })
      .catch((error: unknown) => { if (!cancelled) setState({ fileId: mediaFileId, lyric: null, error: errorMessage(error), loading: false }); });
    return () => { cancelled = true; };
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => load(false), [mediaFileId]);

  const current = state.fileId === mediaFileId ? state : { lyric: null, error: null, loading: true };
  const lines = useMemo(() => (current.lyric?.synced ? parseLrc(current.lyric.syncedLyrics) : []), [current.lyric]);
  const active = activeLine(lines, position);

  // Keep the sung line in view, unless the person has just scrolled.
  const list = useRef<HTMLOListElement>(null);
  const manualUntil = useRef(0);
  useEffect(() => {
    if (active < 0 || Date.now() < manualUntil.current) return;
    const node = list.current?.querySelector<HTMLElement>(`[data-line="${active}"]`);
    if (!node || !list.current) return;
    const box = list.current;
    const top = node.offsetTop - box.clientHeight / 2 + node.clientHeight / 2;
    box.scrollTo({ top: Math.max(0, top), behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
  }, [active]);
  const noteManual = () => { manualUntil.current = Date.now() + MANUAL_SCROLL_MS; };

  const height = fill ? "min-h-0 flex-1" : "h-80";
  if (current.loading) {
    return <div className={cn("flex items-center justify-center gap-3 text-body-sm text-muted", height)}><Spinner size="sm" /> Looking for lyrics…</div>;
  }
  const lyric = current.lyric;
  const credit = lyricsCredit(lyric?.source);
  const again = <Button size="sm" variant="ghost" onClick={() => load(true)}>Look again</Button>;
  if (current.error || !lyric || lyric.source === "none" || (!lines.length && !lyric.plainLyrics && !lyric.instrumental)) {
    return (
      <div className={cn("flex flex-col items-center justify-center gap-3 text-center text-body-sm text-muted", height)}>
        <p>{current.error ?? "No lyrics were found for this track."}</p>
        {again}
      </div>
    );
  }
  if (lyric.instrumental && !lines.length && !lyric.plainLyrics) {
    return <div className={cn("flex items-center justify-center text-body-sm text-muted", height)}>Instrumental</div>;
  }

  return (
    <div className={cn("flex flex-col gap-2", fill ? "min-h-0 flex-1" : undefined)}>
      {lines.length ? (
        <ol
          ref={list}
          aria-label="Lyrics"
          data-nav-group
          className={cn("relative overflow-y-auto overscroll-contain px-(--focus-reach) py-[30%] [scrollbar-width:thin]", height)}
          onWheel={noteManual}
          onTouchMove={noteManual}
          onKeyDown={(event) => { if (event.key === "PageUp" || event.key === "PageDown") noteManual(); }}
        >
          {lines.map((line, index) => (
            <li key={`${line.start}-${index}`} data-line={index}>
              <button
                type="button"
                data-focusable
                data-nav-default={index === Math.max(0, active) ? "" : undefined}
                aria-current={index === active ? "true" : undefined}
                onClick={() => onSeek(line.start)}
                className={cn(
                  "w-full rounded-control px-3 py-1.5 text-start transition-[color,transform] duration-base",
                  "text-title-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
                  index === active ? "text-foreground" : index < active ? "text-muted/70" : "text-muted",
                  tv && "text-title-lg",
                )}
              >
                {line.text || <span aria-label="Instrumental break">♪</span>}
              </button>
            </li>
          ))}
        </ol>
      ) : (
        <div className={cn("overflow-y-auto", height)} data-nav-group tabIndex={0} aria-label="Lyrics">
          {stanzas(lyric.plainLyrics).map((stanza, index) => (
            <p key={index} className="mb-4 text-body text-foreground">
              {stanza.map((line, lineIndex) => <span key={lineIndex} className="block">{line}</span>)}
            </p>
          ))}
        </div>
      )}
      <div className="flex items-center justify-between gap-3 text-label-sm text-muted">
        <span>{credit}{lines.length ? "" : credit ? " · not synced" : "Not synced"}</span>
        {again}
      </div>
    </div>
  );
}
