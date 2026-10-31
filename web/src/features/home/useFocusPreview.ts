import { useCallback, useEffect, useRef, useState, type FocusEvent } from "react";

import type { SpotlightItem } from "./HomeSpotlight";

/** Long enough that holding an arrow key across a row does not reload art for every card. */
const SETTLE_MS = 220;

/** Keyboard or remote focus, as opposed to a pointer click that focuses on its way to a page. */
function focusedByKeys(element: HTMLElement): boolean {
  if (document.documentElement.dataset.inputMode === "tv" || element.dataset.focusVisible === "true") return true;
  try {
    return element.matches(":focus-visible");
  } catch {
    return false;
  }
}

/**
 * A TV home screen's focus preview: the shelf card a remote rests on becomes the page's
 * spotlight, so its art and details fill the screen while the row stays where it is. The
 * preview applies once focus settles on a card and stays when focus moves up to the
 * spotlight, so Play there plays the card that was chosen.
 */
export function useFocusPreview() {
  const [preview, setPreview] = useState<SpotlightItem | null>(null);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  const previewOnFocus = useCallback((item: SpotlightItem) => (event: FocusEvent<HTMLElement>) => {
    const target = event.currentTarget;
    window.clearTimeout(timer.current);
    if (!focusedByKeys(target)) return;
    timer.current = window.setTimeout(() => {
      // Focus may have moved on (into the spotlight, say); only the card still focused counts.
      if (document.activeElement === target) setPreview((current) => (current?.key === item.key ? current : item));
    }, SETTLE_MS);
  }, []);

  const clearPreview = useCallback(() => {
    window.clearTimeout(timer.current);
    setPreview(null);
  }, []);

  return { preview, previewOnFocus, clearPreview };
}
