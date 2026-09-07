import { useEffect, useState, type CSSProperties, type ReactNode } from "react";

import { resolveArtwork } from "@/lib/api/urls";
import { cn } from "@/lib/utils";

export type ArtworkAspect = "poster" | "square" | "backdrop" | "banner";

const ASPECT: Record<ArtworkAspect, string> = {
  poster: "aspect-[2/3]",
  square: "aspect-square",
  backdrop: "aspect-video",
  banner: "aspect-[5/1]",
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
}

/**
 * Artwork with a tinted placeholder, fade-in on load and graceful failure. The placeholder
 * is the same size as the final image so layouts never shift.
 */
export function Artwork({ src, alt, aspect = "poster", className, imageClassName, tint, fallback, priority, style, sizes }: ArtworkProps) {
  const resolved = resolveArtwork(src);
  const [state, setState] = useState<"loading" | "loaded" | "error">(resolved ? "loading" : "error");

  useEffect(() => {
    setState(resolved ? "loading" : "error");
  }, [resolved]);

  return (
    <div
      className={cn("relative overflow-hidden bg-surface-secondary placeholder-gradient", ASPECT[aspect], className)}
      style={{ ...(tint ? ({ "--placeholder-tint": tint } as CSSProperties) : null), ...style }}
    >
      {resolved && state !== "error" ? (
        <img
          src={resolved}
          alt={alt}
          sizes={sizes}
          loading={priority ? "eager" : "lazy"}
          decoding="async"
          fetchPriority={priority ? "high" : "auto"}
          draggable={false}
          onLoad={() => setState("loaded")}
          onError={() => setState("error")}
          className={cn(
            "absolute inset-0 h-full w-full object-cover transition-opacity duration-slow ease-fluid",
            state === "loaded" ? "opacity-100" : "opacity-0",
            imageClassName,
          )}
        />
      ) : null}
      {state === "error" && fallback ? <div className="absolute inset-0 grid place-items-center text-foreground/60">{fallback}</div> : null}
    </div>
  );
}
