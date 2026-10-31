import { useEffect, useState, type CSSProperties } from "react";

import { resolveArtwork, sizedArtwork } from "@/lib/api/urls";
import { cn } from "@/lib/utils";

interface MediaBackdropProps {
  /** Wide artwork. When it is missing or fails, the fallback (normally the poster) is used. */
  src: string | null | undefined;
  fallback?: string | null;
  tint?: string;
  className?: string;
  /** Reports whether wide artwork is shown, so a hero can size to the art or to its text. */
  onWideChange?: (wide: boolean) => void;
}

/**
 * Page-level artwork behind a media hero. The sharp image fades into the canvas, and a
 * heavily blurred copy of the same picture spreads its colour further down the page, so a
 * title's art tints the whole view instead of ending at a hard edge. A poster fallback is
 * shown only as that colour field: a portrait image stretched to a banner looks wrong.
 * Decorative: it never receives pointer or focus, and sits under `.over-imagery` content.
 */
export function MediaBackdrop({ src, fallback, tint, className, onWideChange }: MediaBackdropProps) {
  const primary = resolveArtwork(src);
  const secondary = resolveArtwork(fallback);
  const [failed, setFailed] = useState<ReadonlySet<string>>(() => new Set());
  const [loaded, setLoaded] = useState<string | null>(null);
  const fail = (source: string) => setFailed((previous) => (previous.has(source) ? previous : new Set(previous).add(source)));
  const wide = primary && !failed.has(primary) ? primary : undefined;
  const image = wide ?? (secondary && !failed.has(secondary) ? secondary : undefined);
  // The sharp layer covers the viewport; the wash is blurred to nothing, so a thumbnail does.
  const sharpWidth = typeof window === "undefined" ? 1920 : Math.min(window.innerWidth, 1920);
  const sharpSrc = sizedArtwork(wide, sharpWidth);
  const washSrc = sizedArtwork(image, 320);
  const hasWide = Boolean(wide);
  useEffect(() => onWideChange?.(hasWide), [hasWide, onWideChange]);
  // When the art changes (a remote moving along a shelf), the last picture stays underneath
  // until the new one has faded in over it, instead of dropping to the bare tint between.
  const [current, setCurrent] = useState(wide);
  const [behind, setBehind] = useState<string | null>(null);
  if (wide !== current) {
    setCurrent(wide);
    setBehind(loaded && loaded !== wide ? loaded : null);
  }
  const behindSrc = wide && behind ? sizedArtwork(behind, sharpWidth) : undefined;

  return (
    <div
      aria-hidden
      data-print="hide"
      className={cn("media-backdrop pointer-events-none absolute inset-x-0 top-0 -z-10", className)}
      style={tint ? ({ "--placeholder-tint": tint } as CSSProperties) : undefined}
    >
      <div className="media-backdrop-tint absolute inset-0" />
      {image ? (
        <>
          <img
            key={`wash-${image}`}
            src={washSrc}
            alt=""
            decoding="async"
            draggable={false}
            onError={() => fail(image)}
            className="media-backdrop-wash"
          />
          {behindSrc ? <img key={behind!} src={behindSrc} alt="" decoding="async" draggable={false} data-loaded="true" className="media-backdrop-image" /> : null}
          {wide ? (
            <img
              key={wide}
              src={sharpSrc}
              alt=""
              decoding="async"
              fetchPriority="high"
              draggable={false}
              onLoad={() => setLoaded(wide)}
              onError={() => fail(wide)}
              data-loaded={loaded === wide ? "true" : undefined}
              className="media-backdrop-image"
            />
          ) : null}
        </>
      ) : null}
      <div className="media-backdrop-scrim absolute inset-0" />
    </div>
  );
}
