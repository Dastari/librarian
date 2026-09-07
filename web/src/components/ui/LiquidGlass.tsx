import { useRef, type CSSProperties, type ReactNode } from "react";

import { supportsRefraction, useRefraction, type RefractionOptions } from "@/lib/refraction";
import { cn } from "@/lib/utils";

interface LiquidGlassProps extends RefractionOptions {
  children?: ReactNode;
  className?: string;
  style?: CSSProperties;
  strong?: boolean;
  /** Skip the expensive filter (e.g. many small elements in a list). */
  flat?: boolean;
}

/**
 * Refractive glass surface. The filter layer sits behind the content and bends what is behind
 * the element's edges; browsers without `backdrop-filter: url()` get the flat `.glass` look.
 */
export function LiquidGlass({ children, className, style, strong = false, flat = false, ...options }: LiquidGlassProps) {
  const ref = useRef<HTMLDivElement>(null);
  const filter = useRefraction(ref, options, !flat);
  const refract = supportsRefraction && !flat;
  return (
    <div ref={ref} className={cn("liquid-glass rounded-card glass-highlight", refract ? "border border-glass-border shadow-glass" : strong ? "glass-strong" : "glass", className)} style={style}>
      {refract ? <span aria-hidden className="liquid-glass__filter" style={{ backdropFilter: filter, WebkitBackdropFilter: filter, background: strong ? "var(--glass-fill-strong)" : "var(--glass-fill)" }} /> : null}
      <div className="liquid-glass__content h-full w-full">{children}</div>
    </div>
  );
}
