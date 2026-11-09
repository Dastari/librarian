import { lazy, Suspense, type RefObject } from "react";

import { useAnimatedBackdrop } from "@/lib/animated-backdrop";
import { cn } from "@/lib/utils";

const FilmConstellation = lazy(() => import("./FilmConstellation").then((module) => ({ default: module.FilmConstellation })));

/**
 * Sign-in backdrop: the ambient gradient, the film constellation across the whole viewport and
 * a soft vignette so the drawing recedes towards the edges and behind the card. The constellation
 * is a full-screen canvas redrawn every frame, so it follows the same rules as the Home scene
 * (`useAnimatedBackdrop`): only the gradient in the TV app and on low-memory devices.
 */
export function ConstellationBackdrop({ className, clearRef }: { className?: string; clearRef?: RefObject<HTMLElement | null> }) {
  const animated = useAnimatedBackdrop({ screen: false });
  return (
    <div className={cn("pointer-events-none absolute inset-0 -z-10 overflow-hidden", className)} aria-hidden>
      <div className="canvas-ambient absolute inset-0" />
      {animated ? (
        <Suspense fallback={null}>
          <FilmConstellation className="absolute inset-0 size-full" opacity={0.62} clearRef={clearRef} />
        </Suspense>
      ) : null}
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,transparent_30%,var(--background)_125%)] opacity-75" />
      <div className="absolute inset-0 bg-gradient-to-t from-background/60 via-transparent to-background/20" />
    </div>
  );
}
