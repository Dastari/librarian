import { Link, useMatches } from "@tanstack/react-router";
import { IconChevronRight } from "@tabler/icons-react";
import { Fragment } from "react";

import { cn } from "@/lib/utils";

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

export function Breadcrumbs({ className }: { className?: string }) {
  const crumbs = useCrumbs();
  if (crumbs.length === 0) return null;
  return (
    <nav aria-label="Breadcrumb" className={cn("min-w-0", className)}>
      <ol className="flex min-w-0 items-center gap-1 text-body-sm">
        {crumbs.map((crumb, index) => {
          const last = index === crumbs.length - 1;
          return (
            <Fragment key={`${crumb.label}-${index}`}>
              {index > 0 ? <IconChevronRight size={14} className="shrink-0 text-muted/60" aria-hidden /> : null}
              <li className={cn("min-w-0 truncate", last ? "text-foreground" : "text-muted", index < crumbs.length - 2 && "hidden sm:block")}>
                {crumb.href && !last ? (
                  <Link to={crumb.href as never} className="nav-focus rounded-md px-1 py-0.5 transition-colors hover:text-foreground" data-focusable>
                    {crumb.label}
                  </Link>
                ) : (
                  <span className="px-1 py-0.5" aria-current={last ? "page" : undefined}>
                    {crumb.label}
                  </span>
                )}
              </li>
            </Fragment>
          );
        })}
      </ol>
    </nav>
  );
}
