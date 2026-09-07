import { Tooltip } from "@heroui/react";

import { BrandMark } from "@/components/ui";
import { Link, useRouterState } from "@tanstack/react-router";
import { IconChevronsLeft, IconChevronsRight } from "@tabler/icons-react";
import { useQuery } from "@apollo/client/react";

import { NavLibrariesDocument } from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { libraryType } from "@/lib/library-types";
import { usePref } from "@/lib/prefs";
import { cn } from "@/lib/utils";

import { PRIMARY_NAV, isNavActive } from "./nav";
import { useShellCounts } from "./useShellCounts";

/**
 * Left navigation rail for tablet, desktop and TV. Collapsed by default (icons), expands to
 * show labels and the library list. State persists per device.
 */
export function NavRail() {
  const [expanded, setExpanded] = usePref("shell.railExpanded", false);
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const counts = useShellCounts();
  const isAdmin = useIsAdmin();
  const { status } = useSession();
  const libraries = useQuery(NavLibrariesDocument, { skip: status !== "authenticated" });

  return (
    <aside
      aria-label="Primary"
      data-expanded={expanded}
      className={cn(
        "glass-chrome sticky top-0 z-30 hidden h-svh shrink-0 flex-col rounded-none border-0 border-r pt-[calc(var(--safe-top)+0.75rem)] pb-[calc(var(--safe-bottom)+0.75rem)] transition-[width] duration-base ease-fluid md:flex",
        expanded ? "w-rail-expanded" : "w-rail",
      )}
    >
      <Link to="/" className="nav-focus mx-auto mb-4 flex h-11 items-center gap-3 rounded-xl px-2" data-focusable aria-label="Home">
        <BrandMark size={30} />
        {expanded ? <span className="text-title-md font-display tracking-tight text-foreground">Librarian</span> : null}
      </Link>

      <nav className="flex flex-col gap-1 px-2">
        {PRIMARY_NAV.filter((item) => !item.adminOnly || isAdmin).map((item) => {
          const active = isNavActive(item.href, pathname);
          const badge = item.badge === "downloads" ? counts.downloads : item.badge === "notifications" ? counts.notifications : 0;
          const link = (
            <Link
              to={item.href as never}
              data-focusable
              aria-current={active ? "page" : undefined}
              className={cn(
                "nav-focus relative flex h-11 items-center gap-3 rounded-xl px-3 text-rail-foreground transition-colors duration-fast",
                active ? "bg-brand-soft text-foreground" : "hover:bg-surface-hover hover:text-foreground",
                !expanded && "justify-center px-0",
              )}
            >
              <item.icon size={22} stroke={1.75} className={cn("shrink-0", active && "text-brand")} />
              {expanded ? <span className="truncate text-body-sm font-medium">{item.label}</span> : null}
              {badge > 0 ? (
                <span
                  className={cn(
                    "text-numeric grid min-w-5 place-items-center rounded-full bg-brand px-1.5 text-label-sm leading-5 text-brand-foreground",
                    expanded ? "ml-auto" : "absolute right-1 top-1 min-w-4 text-[10px] leading-4",
                  )}
                >
                  {badge > 99 ? "99+" : badge}
                </span>
              ) : null}
            </Link>
          );
          if (expanded) return <div key={item.key}>{link}</div>;
          return (
            <Tooltip key={item.key} delay={300} closeDelay={0}>
              {link}
              <Tooltip.Content placement="right">{item.label}</Tooltip.Content>
            </Tooltip>
          );
        })}
      </nav>

      {libraries.data?.libraries.edges.length ? (
        <div className="mt-4 flex min-h-0 flex-1 flex-col px-2">
          {expanded ? <p className="text-overline mb-1.5 px-3 text-muted">Libraries</p> : <div className="mx-3 mb-2 h-px bg-separator" />}
          <div className="scrollbar-none flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto">
            {libraries.data.libraries.edges.map(({ node }) => {
              const meta = libraryType(node.libraryType);
              const href = `/libraries/${node.id}`;
              const active = isNavActive(href, pathname);
              const link = (
                <Link
                  to="/libraries/$libraryId"
                  params={{ libraryId: node.id }}
                  data-focusable
                  aria-current={active ? "page" : undefined}
                  className={cn(
                    "nav-focus flex h-10 items-center gap-3 rounded-xl px-3 text-rail-foreground transition-colors duration-fast",
                    active ? "bg-surface-hover text-foreground" : "hover:bg-surface-hover hover:text-foreground",
                    !expanded && "justify-center px-0",
                  )}
                >
                  <meta.icon size={20} stroke={1.75} className={cn("shrink-0", meta.tint, node.scanning && "animate-pulse-soft")} />
                  {expanded ? <span className="truncate text-body-sm">{node.name}</span> : null}
                </Link>
              );
              if (expanded) return <div key={node.id}>{link}</div>;
              return (
                <Tooltip key={node.id} delay={300} closeDelay={0}>
                  {link}
                  <Tooltip.Content placement="right">{node.name}</Tooltip.Content>
                </Tooltip>
              );
            })}
          </div>
        </div>
      ) : (
        <div className="flex-1" />
      )}

      <button
        type="button"
        data-focusable
        onClick={() => setExpanded((value) => !value)}
        aria-label={expanded ? "Collapse navigation" : "Expand navigation"}
        className="nav-focus mx-2 mt-2 flex h-10 items-center justify-center rounded-xl text-muted transition-colors duration-fast hover:bg-surface-hover hover:text-foreground"
      >
        {expanded ? <IconChevronsLeft size={20} /> : <IconChevronsRight size={20} />}
      </button>
    </aside>
  );
}
