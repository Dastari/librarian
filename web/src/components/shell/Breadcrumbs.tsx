import { useMatches } from "@tanstack/react-router";
import { RouteBreadcrumb } from "glass-ux/breadcrumb";

export interface Crumb {
  label: string;
  href?: string;
}

/**
 * Context-aware breadcrumbs derived from the matched route tree. Routes declare a crumb through
 * `staticData.crumb` (static label) or by returning `{ crumb }` from their loader (dynamic label,
 * e.g. the movie title). The current page is the last, unlinked crumb.
 */
export function useCrumbs(): Crumb[] {
  const matches = useMatches();
  const crumbs: Crumb[] = [];
  for (const match of matches) {
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
