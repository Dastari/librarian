/**
 * Shared-element morph for opening media: the activated tile's artwork takes the name the
 * detail hero's poster carries (`media-poster`, see media.css), so the router's view
 * transition grows the tile into the hero. Names must be unique while a snapshot is taken;
 * anything else holding the name gives it up briefly, then the inline overrides are cleared.
 */
const MORPH = "media-poster";
const CLEAR_AFTER_MS = 1200;

export function claimMorph(element: Element | null | undefined) {
  if (!(element instanceof HTMLElement) || typeof document.startViewTransition !== "function") return;
  const released: HTMLElement[] = [];
  for (const other of document.querySelectorAll<HTMLElement>(".media-hero-poster, [data-morph-claimed]")) {
    if (other === element) continue;
    other.style.viewTransitionName = "none";
    delete other.dataset.morphClaimed;
    released.push(other);
  }
  element.style.viewTransitionName = MORPH;
  element.dataset.morphClaimed = "true";
  window.setTimeout(() => {
    for (const other of released) other.style.removeProperty("view-transition-name");
    element.style.removeProperty("view-transition-name");
    delete element.dataset.morphClaimed;
  }, CLEAR_AFTER_MS);
}

/** Sections whose layout (heading, tabs, side menu) stays on screen while its tabs change. */
const TABBED_SECTIONS = [/^\/settings\//, /^\/libraries\/[^/]+\//];

const sectionOf = (pathname: string) => TABBED_SECTIONS.map((pattern) => pathname.match(pattern)?.[0]).find(Boolean);

/**
 * Route view transitions for the router: a new page fades in, but switching tabs within one
 * section, or changing only the search (filters, sorting), keeps the layout still and swaps
 * the content in place. Animating the whole view there replays the menu and heading.
 */
export function routeTransitionTypes({ fromLocation, toLocation, pathChanged }: { fromLocation?: { pathname: string }; toLocation: { pathname: string }; pathChanged: boolean }): string[] | false {
  if (!pathChanged) return false;
  const from = fromLocation ? sectionOf(fromLocation.pathname) : undefined;
  if (from && from === sectionOf(toLocation.pathname)) return false;
  return ["route"];
}
