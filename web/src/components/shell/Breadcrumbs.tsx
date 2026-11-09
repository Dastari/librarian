import { useMatches } from "@tanstack/react-router";
import { RouteBreadcrumb } from "glass-ux/breadcrumb";

export interface Crumb {
  label: string;
  href?: string;
}

/** The library tab a detail page's item is listed under, and the label that tab goes by. */
const LIBRARY_TABS = {
  movies: "Browse",
  shows: "Browse",
  albums: "Albums",
  artists: "Artists",
  audiobooks: "Audiobooks",
} as const;

/**
 * The trail above an item that has a page of its own outside the library routes (a movie, a show,
 * an album): Libraries, its library, and the tab that lists it. A loader returns it as `trail`.
 */
export function libraryTrail(library: { id: string; name: string } | null | undefined, tab: keyof typeof LIBRARY_TABS): Crumb[] {
  if (!library) return [];
  const base = `/libraries/${library.id}`;
  return [
    { label: "Libraries", href: "/libraries" },
    { label: library.name, href: base },
    { label: LIBRARY_TABS[tab], href: `${base}/${tab}` },
  ];
}

/**
 * Context-aware breadcrumbs derived from the matched route tree. Routes declare a crumb through
 * `staticData.crumb` (static label) or by returning `{ crumb }` from their loader (dynamic label,
 * e.g. the movie title). A page that lives outside its library's routes also returns `{ trail }`,
 * the crumbs above it (`libraryTrail`). The current page is the last, unlinked crumb.
 */
export function useCrumbs(): Crumb[] {
  const matches = useMatches();
  const crumbs: Crumb[] = [];
  for (const match of matches) {
    const trail = (match.loaderData as { trail?: Crumb[] } | undefined)?.trail;
    for (const crumb of trail ?? []) if (!crumbs.some((existing) => existing.href === crumb.href)) crumbs.push(crumb);
    const loaderCrumb = (match.loaderData as { crumb?: string } | undefined)?.crumb;
    const staticCrumb = match.staticData?.crumb;
    const label = loaderCrumb ?? (typeof staticCrumb === "function" ? staticCrumb(match.params as Record<string, string>) : staticCrumb);
    if (!label) continue;
    crumbs.push({ label, href: match.pathname });
  }
  if (crumbs.length > 0) crumbs[crumbs.length - 1]!.href = undefined;
  return crumbs;
}

/** The route's one breadcrumb, in the top bar (R6.4); it measures and collapses itself. */
export function Breadcrumbs({ className }: { className?: string }) {
  const crumbs = useCrumbs();
  if (crumbs.length === 0) return null;
  return <RouteBreadcrumb className={className} items={crumbs.map((crumb, index) => ({ id: `${index}-${crumb.label}`, label: crumb.label, link: crumb.href ? { to: crumb.href as never } : undefined }))} />;
}
