import { AspectRatio } from "glass-ux/aspect-ratio";
import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";

import { resolveArtwork, sizedArtwork } from "@/lib/api/urls";
import { cn } from "@/lib/utils";

export type ArtworkAspect = "poster" | "square" | "backdrop" | "banner";

const ASPECT: Record<ArtworkAspect, number> = {
  poster: 2 / 3,
  square: 1,
  backdrop: 16 / 9,
  banner: 5,
};

interface ArtworkProps {
  src: string | null | undefined;
  alt: string;
  aspect?: ArtworkAspect;
  className?: string;
  imageClassName?: string;
  /** Media tint for the placeholder gradient. */
  tint?: string;
  /** Rendered centred over the placeholder when no image is available. */
  fallback?: ReactNode;
  priority?: boolean;
  style?: CSSProperties;
  sizes?: string;
  /** Displayed width in CSS pixels: cached artwork is fetched at a matching stored size. */
  displayWidth?: number;
}

/**
 * Artwork with a tinted placeholder, fade-in on load and graceful failure. The placeholder
 * is the same size as the final image so layouts never shift.
 */
export function Artwork({ src, alt, aspect = "poster", className, imageClassName, tint, fallback, priority, style, sizes, displayWidth }: ArtworkProps) {
  const resolved = sizedArtwork(resolveArtwork(src), displayWidth);
  // State belongs to one source. A cached image can finish loading during commit, before
  // any effect runs, so a reset effect would hide an image that has already arrived.
  const [result, setResult] = useState<{ src: string; state: "loaded" | "error" } | null>(null);
  const state = !resolved ? "error" : result?.src === resolved ? result.state : "loading";
  const settle = (next: "loaded" | "error") => resolved && setResult({ src: resolved, state: next });
  const image = useRef<HTMLImageElement>(null);

  useEffect(() => {
    const element = image.current;
    if (element?.complete && resolved) settle(element.naturalWidth > 0 ? "loaded" : "error");
    // Only a new source needs re-checking; the handlers cover later loads.
  }, [resolved]);

  return (
    <AspectRatio
      ratio={ASPECT[aspect]}
      className={cn("bg-surface-secondary placeholder-gradient", className)}
      style={{ ...(tint ? ({ "--placeholder-tint": tint } as CSSProperties) : null), ...style }}
    >
      {resolved && state !== "error" ? (
        <img
          ref={image}
          key={resolved}
          src={resolved}
          alt={alt}
          sizes={sizes}
          loading={priority ? "eager" : "lazy"}
          decoding="async"
          fetchPriority={priority ? "high" : "auto"}
          draggable={false}
          onLoad={() => settle("loaded")}
          onError={() => settle("error")}
          className={cn(
            "absolute inset-0 h-full w-full object-cover transition-opacity duration-slow ease-fluid",
            state === "loaded" ? "opacity-100" : "opacity-0",
            imageClassName,
          )}
        />
      ) : null}
      {state === "error" && fallback ? <div className="absolute inset-0 grid place-items-center text-foreground/60">{fallback}</div> : null}
    </AspectRatio>
  );
}
