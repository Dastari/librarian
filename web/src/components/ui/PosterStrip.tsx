import { useState } from "react";

import { sizedArtwork } from "@/lib/api/urls";
import { cn } from "@/lib/utils";

interface PosterStripProps {
  images: string[];
  className?: string;
  /** Poster height in px; width follows the 2:3 ratio. */
  height?: number;
  square?: boolean;
  /** Most posters to show. */
  limit?: number;
}

/**
 * A still, slightly fanned strip of a library's artwork behind a tile's glass band. Images
 * that fail are dropped rather than showing a broken-image glyph, and nothing loops (R18.2).
 */
export function PosterStrip({ images, className, height = 120, square, limit = 6 }: PosterStripProps) {
  const [failed, setFailed] = useState<ReadonlySet<string>>(() => new Set());
  const shown = images.filter((src) => !failed.has(src)).slice(0, limit);
  if (shown.length === 0) return null;
  const width = square ? height : Math.round((height * 2) / 3);
  return (
    <div aria-hidden className={cn("pointer-events-none absolute inset-0 overflow-hidden", className)}>
      <div className="flex h-full items-center gap-2 ps-[22%]">
        {shown.map((src, index) => (
          <img
            key={src}
            src={sizedArtwork(src, width)}
            alt=""
            loading="lazy"
            decoding="async"
            draggable={false}
            onError={() => setFailed((previous) => new Set(previous).add(src))}
            className="shrink-0 rounded-md object-cover shadow-poster"
            style={{ width, height, transform: `rotate(${index % 2 ? -3 : 3}deg) translateY(${index % 3 === 0 ? -8 : index % 3 === 1 ? 6 : 0}px)` }}
          />
        ))}
      </div>
    </div>
  );
}
