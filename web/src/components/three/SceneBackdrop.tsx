import { Suspense, lazy } from "react";

import { useAnimatedBackdrop } from "@/lib/animated-backdrop";
import { cn } from "@/lib/utils";

const AmbientScene = lazy(() => import("./AmbientScene"));

interface SceneBackdropProps {
  className?: string;
  density?: number;
  /** Optional cover images to float as panes. */
  images?: string[];
}

/**
 * Wraps the 3D scene with a gradient that always paints first. WebGL (and its chunk) is skipped
 * entirely in the TV app, on 2 GB devices, for reduced motion, on small screens and when this
 * device turned animated backgrounds off (`useAnimatedBackdrop`).
 */
export function SceneBackdrop({ className, density, images }: SceneBackdropProps) {
  const enabled = useAnimatedBackdrop();

  return (
    <div className={cn("pointer-events-none absolute inset-0 -z-10 overflow-hidden", className)} aria-hidden>
      <div className="canvas-ambient absolute inset-0" />
      {enabled ? (
        <Suspense fallback={null}>
          <AmbientScene density={density} images={images} />
        </Suspense>
      ) : null}
      <div className="absolute inset-0 bg-gradient-to-t from-background/70 via-transparent to-background/30" />
    </div>
  );
}
