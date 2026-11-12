import { useEffect, useRef, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";

import { registerShellArt, SHELL_ART_ID } from "@/components/shell/shellArt";
import { resolveArtwork, sizedArtwork } from "@/lib/api/urls";
import { cn } from "@/lib/utils";

/** How wide the wash is drawn: it is only a field of colour, so a thumbnail is plenty. */
const WASH_WIDTH = 48;

/**
 * The colour wash: the artwork drawn once into a canvas a few dozen pixels wide, lightly blurred,
 * and stretched over the page by CSS. Stretching a tiny image blurs it the rest of the way for
 * free, where a CSS `filter: blur()` over the full-size picture made the GPU blur a whole screen
 * of pixels each time the page drew it (design.md Q155).
 */
function BackdropWash({ src, onError }: { src: string; onError: () => void }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [drawn, setDrawn] = useState(false);
  useEffect(() => {
    let cancelled = false;
    const picture = new Image();
    picture.decoding = "async";
    picture.onload = () => {
      const target = canvas.current;
      const context = target?.getContext("2d");
      if (cancelled || !target || !context || !picture.naturalWidth) return;
      target.width = WASH_WIDTH;
      target.height = Math.max(1, Math.round((WASH_WIDTH * picture.naturalHeight) / picture.naturalWidth));
      // Browsers without canvas filters still get a soft field: the stretch does most of the work.
      context.filter = "blur(1.5px) saturate(1.4)";
      context.drawImage(picture, 0, 0, target.width, target.height);
      // Its fade to the canvas is baked in here rather than a CSS mask: a mask is one more
      // full-screen offscreen pass every time the backdrop moves.
      context.filter = "none";
      const fade = context.createLinearGradient(0, 0, 0, target.height);
      fade.addColorStop(0.25, "#000");
      fade.addColorStop(1, "transparent");
      context.globalCompositeOperation = "destination-in";
      context.fillStyle = fade;
      context.fillRect(0, 0, target.width, target.height);
      setDrawn(true);
    };
    picture.onerror = () => {
      if (!cancelled) onError();
    };
    picture.src = src;
    return () => {
      cancelled = true;
    };
    // `onError` is a fresh closure each render; the picture to draw is what matters.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [src]);
  return <canvas ref={canvas} aria-hidden data-loaded={drawn ? "true" : undefined} className="media-backdrop-wash" />;
}

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
 * Page-level artwork behind a media hero and the shell. The sharp image fades into the canvas, and a
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
  const washSrc = sizedArtwork(image, WASH_WIDTH);
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
  useEffect(() => registerShellArt(), []);

  const layer = (
    <div
      aria-hidden
      data-print="hide"
      className={cn("media-backdrop pointer-events-none absolute inset-0", className)}
      style={tint ? ({ "--placeholder-tint": tint } as CSSProperties) : undefined}
    >
      <div className="media-backdrop-tint absolute inset-0" />
      {image ? (
        <>
          {washSrc ? <BackdropWash key={`wash-${image}`} src={washSrc} onError={() => fail(image)} /> : null}
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
  // Behind the whole shell, so the picture reaches under the rail and the top bar.
  const target = typeof document === "undefined" ? null : document.getElementById(SHELL_ART_ID);
  return target ? createPortal(layer, target) : layer;
}
