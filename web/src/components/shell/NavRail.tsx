import { Sidebar, useSidebar } from "glass-ux/sidebar";
import { Link, useRouterState } from "@tanstack/react-router";
import { IconChevronsLeft, IconChevronsRight } from "@tabler/icons-react";

import { BrandMark } from "@/components/ui";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { useLibraryPreferences } from "@/features/personal/usePersonalLibrary";
import { EntityLibraryListDocument } from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { useInputMode } from "@/lib/input-mode";
import { libraryType } from "@/lib/library-types";
import { cn } from "@/lib/utils";

import { PRIMARY_NAV, isNavActive } from "./nav";
import { useShellCounts } from "./useShellCounts";

/**
 * Left navigation for tablet, desktop and TV, as a strip of icons that expands to labels and
 * the library list. The zone attributes put it in the remote's spatial navigation; a remote
 * has no use for the resize handle, so TVs get the toggle at the foot and nothing else.
 */
export function NavRail() {
  const { open, toggle } = useSidebar();
  const tv = useInputMode().mode === "tv";
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const counts = useShellCounts();
  const isAdmin = useIsAdmin();
  const { status } = useSession();
  const libraries = useCompleteConnection(EntityLibraryListDocument, {}, (data) => data.libraries, { skip: status !== "authenticated" });
  const preferences = useLibraryPreferences();
  const favouriteIds = new Set(preferences.rows.filter((row) => row.favourite).map((row) => row.libraryId));
  const libraryRows = [...libraries.rows].sort((a, b) => Number(favouriteIds.has(b.id)) - Number(favouriteIds.has(a.id)));
  const ToggleIcon = open ? IconChevronsLeft : IconChevronsRight;

  return (
    <Sidebar collapsible="icon" resizable={!tv} aria-label="Primary" className="app-shell-rail" data-nav-zone="rail" data-nav-side="" data-print="hide">
      <Sidebar.Header
        toggle={false}
        icon={<Link to="/" className="nav-focus rounded-xl" data-focusable aria-label="Home"><BrandMark size={30} /></Link>}
        title={<span className="text-title-md font-display tracking-tight text-foreground">Librarian</span>}
      />
      <Sidebar.Content>
        <Sidebar.Group>
          <Sidebar.Menu>
            {PRIMARY_NAV.filter((item) => !item.adminOnly || isAdmin).map((item) => {
              const badge = item.badge === "downloads" ? counts.downloads : item.badge === "notifications" ? counts.notifications : 0;
              return (
                <Sidebar.MenuItem key={item.key}>
                  <Sidebar.MenuButton className="nav-focus" icon={<item.icon stroke={1.75} />} active={isNavActive(item.href, pathname)} render={<Link to={item.href as never} data-focusable />}>
                    {item.label}
                  </Sidebar.MenuButton>
                  {badge > 0 ? <Sidebar.MenuBadge value={badge} /> : null}
                </Sidebar.MenuItem>
              );
            })}
          </Sidebar.Menu>
        </Sidebar.Group>
        {libraryRows.length ? (
          <Sidebar.Group>
            <Sidebar.GroupLabel>Libraries</Sidebar.GroupLabel>
            <Sidebar.Menu>
              {libraryRows.map((node) => {
                const meta = libraryType(node.libraryType);
                return (
                  <Sidebar.MenuItem key={node.id}>
                    <Sidebar.MenuButton
                      className="nav-focus"
                      icon={<meta.icon stroke={1.75} className={cn(meta.tint, node.scanning && "animate-pulse-soft")} />}
                      active={isNavActive(`/libraries/${node.id}`, pathname)}
                      render={<Link to="/libraries/$libraryId" params={{ libraryId: node.id }} data-focusable />}
                    >
                      {node.name}
                    </Sidebar.MenuButton>
                  </Sidebar.MenuItem>
                );
              })}
            </Sidebar.Menu>
          </Sidebar.Group>
        ) : null}
      </Sidebar.Content>
      <Sidebar.Footer>
        <Sidebar.Menu>
          <Sidebar.MenuItem>
            <Sidebar.MenuButton className="nav-focus" data-focusable icon={<ToggleIcon className="rtl:rotate-180" />} onClick={toggle}>
              {open ? "Collapse navigation" : "Expand navigation"}
            </Sidebar.MenuButton>
          </Sidebar.MenuItem>
        </Sidebar.Menu>
      </Sidebar.Footer>
      {tv ? null : <Sidebar.Rail />}
    </Sidebar>
  );
}
