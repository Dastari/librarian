import { Suspense, lazy, useEffect, useState } from "react";

import { cn } from "@/lib/utils";

const AmbientScene = lazy(() => import("./AmbientScene"));

interface SceneBackdropProps {
  className?: string;
  density?: number;
  /** Optional cover images to float as panes. */
  images?: string[];
}

/**
 * Wraps the 3D scene with a gradient that always paints first, and skips WebGL entirely for
 * reduced-motion users, very small screens and devices that report low memory.
 */
export function SceneBackdrop({ className, density, images }: SceneBackdropProps) {
  const [enabled, setEnabled] = useState(false);

  useEffect(() => {
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const small = window.innerWidth < 640;
    const lowMemory = (navigator as Navigator & { deviceMemory?: number }).deviceMemory !== undefined && (navigator as Navigator & { deviceMemory?: number }).deviceMemory! < 2;
    setEnabled(!reduced && !small && !lowMemory);
  }, []);

  return (
    <div className={cn("pointer-events-none absolute inset-0 -z-10 overflow-hidden", className)} aria-hidden>
      <div className="ambient-canvas absolute inset-0" />
      {enabled ? (
        <Suspense fallback={null}>
          <AmbientScene density={density} images={images} />
        </Suspense>
      ) : null}
      <div className="absolute inset-0 bg-gradient-to-t from-background/70 via-transparent to-background/30" />
    </div>
  );
}
