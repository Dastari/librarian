import { useMemo } from "react";

import { cn } from "@/lib/utils";

interface PosterMarqueeProps {
  images: string[];
  className?: string;
  /** Seconds for one full loop. */
  duration?: number;
  /** Poster height in px; width follows the 2:3 ratio. */
  height?: number;
  square?: boolean;
}

/**
 * A slow, seamless strip of artwork used as a living background behind glass. The strip is
 * duplicated so the loop never shows a seam; motion stops for reduced-motion users.
 */
export function PosterMarquee({ images, className, duration = 60, height = 120, square }: PosterMarqueeProps) {
  const strip = useMemo(() => (images.length ? [...images, ...images] : []), [images]);
  if (strip.length === 0) return null;
  const width = square ? height : Math.round((height * 2) / 3);
  return (
    <div aria-hidden className={cn("pointer-events-none absolute inset-0 overflow-hidden", className)}>
      <div className="flex h-full items-center gap-2 motion-safe:animate-[marquee_var(--marquee-duration)_linear_infinite]" style={{ ["--marquee-duration" as string]: `${duration}s`, width: "max-content" }}>
        {strip.map((src, index) => (
          <img key={`${src}-${index}`} src={src} alt="" loading="lazy" decoding="async" draggable={false} className="shrink-0 rounded-md object-cover" style={{ width, height, transform: `rotate(${index % 2 ? -3 : 3}deg) translateY(${index % 3 === 0 ? -8 : index % 3 === 1 ? 6 : 0}px)` }} />
        ))}
      </div>
    </div>
  );
}
