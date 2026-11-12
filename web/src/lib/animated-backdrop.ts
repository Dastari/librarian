import { useEffect, useState, useSyncExternalStore } from "react";

import { animatedBackgroundsStore, animatedBackgroundsUnavailable } from "@/lib/glass/appearance";

/** Narrower than this, the animated backgrounds are skipped: a phone gains nothing from them. */
const SMALL_SCREEN = 640;

/**
 * Whether an animated background (WebGL scene, film constellation) may mount here. False until
 * checked after mount, so the lazy chunk is never fetched where it will not run: never in the TV
 * app or on a 2 GB device, nor on a device that turned them off. `screen` (on by default) also
 * skips reduced motion and small screens; the sign-in constellation draws a still picture for
 * reduced motion itself and stays on phones, so it passes false.
 */
export function useAnimatedBackdrop({ screen = true }: { screen?: boolean } = {}): boolean {
  const allowed = useSyncExternalStore(animatedBackgroundsStore.subscribe, animatedBackgroundsStore.getSnapshot, () => false);
  const [fits, setFits] = useState(false);
  useEffect(() => {
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    setFits(!screen || (!reduced && window.innerWidth >= SMALL_SCREEN));
  }, [screen]);
  return allowed && fits && animatedBackgroundsUnavailable() === null;
}
